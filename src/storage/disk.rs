use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use bytes::Bytes;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::{BoxStream, FileMeta, StorageEngine, StoredFile};
use crate::{MulterError, StorageError};

type CustomFilenameFn = dyn Fn(String) -> String + Send + Sync;
type FileFilterFn = dyn Fn(&FileMeta) -> bool + Send + Sync;

/// Strategy used to derive the final stored filename.
#[derive(Clone)]
pub enum FilenameStrategy {
    /// Keep the incoming filename after sanitization.
    Keep,
    /// Always generate a random filename.
    Random,
    /// Apply a user-provided filename transform.
    Custom(Arc<CustomFilenameFn>),
}

impl fmt::Debug for FilenameStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keep => f.write_str("Keep"),
            Self::Random => f.write_str("Random"),
            Self::Custom(_) => f.write_str("Custom(<fn>)"),
        }
    }
}

/// Builder for [`DiskStorage`].
#[derive(Clone)]
pub struct DiskStorageBuilder {
    root: PathBuf,
    strategy: FilenameStrategy,
    filter: Option<Arc<FileFilterFn>>,
}

impl fmt::Debug for DiskStorageBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiskStorageBuilder")
            .field("root", &self.root)
            .field("strategy", &self.strategy)
            .field("filter", &self.filter.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl DiskStorageBuilder {
    /// Sets the directory used for persisted files.
    pub fn destination(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = root.into();
        self
    }

    /// Alias for [`DiskStorageBuilder::destination`].
    pub fn path(self, root: impl Into<PathBuf>) -> Self {
        self.destination(root)
    }

    /// Sets how output filenames are generated.
    pub fn filename(mut self, strategy: FilenameStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Alias for [`DiskStorageBuilder::filename`].
    pub fn filename_strategy(self, strategy: FilenameStrategy) -> Self {
        self.filename(strategy)
    }

    /// Sets a custom filename function.
    pub fn custom_filename<F>(mut self, transform: F) -> Self
    where
        F: Fn(String) -> String + Send + Sync + 'static,
    {
        self.strategy = FilenameStrategy::Custom(Arc::new(transform));
        self
    }

    /// Sets an optional filter to accept or reject files before persistence.
    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&FileMeta) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(filter));
        self
    }

    /// Builds a validated disk storage backend.
    pub fn build(self) -> Result<DiskStorage, StorageError> {
        if self.root.as_os_str().is_empty() {
            return Err(StorageError::new("disk storage root path cannot be empty"));
        }

        Ok(DiskStorage {
            root: self.root,
            strategy: self.strategy,
            filter: self.filter,
        })
    }
}

impl Default for DiskStorageBuilder {
    fn default() -> Self {
        Self {
            root: std::env::temp_dir().join("rust-multer"),
            strategy: FilenameStrategy::Random,
            filter: None,
        }
    }
}

/// Disk-backed storage engine writing files under a configured root path.
#[derive(Clone)]
pub struct DiskStorage {
    root: PathBuf,
    strategy: FilenameStrategy,
    filter: Option<Arc<FileFilterFn>>,
}

impl fmt::Debug for DiskStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiskStorage")
            .field("root", &self.root)
            .field("strategy", &self.strategy)
            .field("filter", &self.filter.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl DiskStorage {
    /// Creates a disk storage builder.
    pub fn builder() -> DiskStorageBuilder {
        DiskStorageBuilder::default()
    }

    fn choose_output_name(&self, file_name: Option<&str>) -> String {
        let input_name = file_name.map(ToOwned::to_owned).unwrap_or_else(random_basename);

        let candidate = match &self.strategy {
            FilenameStrategy::Keep => input_name,
            FilenameStrategy::Random => random_basename(),
            FilenameStrategy::Custom(transform) => transform(input_name),
        };

        sanitize_filename(&candidate)
    }

    fn should_store(&self, meta: &FileMeta) -> bool {
        self.filter.as_ref().map_or(true, |filter| filter(meta))
    }
}

#[async_trait::async_trait]
impl StorageEngine for DiskStorage {
    type Output = StoredFile;
    type Error = StorageError;

    async fn store(
        &self,
        field_name: &str,
        file_name: Option<&str>,
        content_type: &str,
        mut stream: BoxStream<'_, Result<Bytes, MulterError>>,
    ) -> Result<Self::Output, Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!(
            field_name = field_name,
            file_name = file_name.unwrap_or("<none>"),
            content_type = content_type,
            root = %self.root.display(),
            "disk storage: begin streaming store"
        );

        let accepted_meta = FileMeta {
            field_name: field_name.to_owned(),
            file_name: file_name.map(ToOwned::to_owned),
            content_type: content_type.to_owned(),
        };
        if !self.should_store(&accepted_meta) {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                field_name = field_name,
                file_name = file_name.unwrap_or("<none>"),
                "disk storage filter rejected file"
            );
            return Err(StorageError::new(format!(
                "disk storage filter rejected file field `{field_name}`"
            )));
        }

        tokio::fs::create_dir_all(&self.root)
            .await
            .map_err(|err| StorageError::new(format!("failed to create storage directory: {err}")))?;

        let file_basename = self.choose_output_name(file_name);

        let mut output_path = self.root.join(file_basename);
        if tokio::fs::try_exists(&output_path)
            .await
            .map_err(|err| StorageError::new(format!("failed to inspect output path: {err}")))?
        {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                path = %output_path.display(),
                "disk storage: collision detected, adding suffix"
            );
            output_path = with_collision_suffix(&output_path);
        }

        let mut file = tokio::fs::File::create(&output_path)
            .await
            .map_err(|err| StorageError::new(format!("failed to create output file: {err}")))?;

        let mut written = 0u64;

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|err| StorageError::new(format!("stream read failed: {err}")))?;
            file.write_all(&bytes)
                .await
                .map_err(|err| StorageError::new(format!("failed to write output file: {err}")))?;
            written = written.saturating_add(bytes.len() as u64);
        }

        file.flush()
            .await
            .map_err(|err| StorageError::new(format!("failed to flush output file: {err}")))?;

        let storage_key = output_path.to_string_lossy().into_owned();
        let parsed_content_type = content_type
            .parse::<mime::Mime>()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM);
        #[cfg(feature = "tracing")]
        tracing::debug!(
            field_name = field_name,
            size = written,
            path = %output_path.display(),
            "disk storage: completed store"
        );
        Ok(StoredFile {
            storage_key,
            field_name: field_name.to_owned(),
            file_name: file_name.map(ToOwned::to_owned),
            content_type: parsed_content_type,
            size: written,
            path: Some(output_path),
        })
    }
}

fn random_basename() -> String {
    Uuid::new_v4().simple().to_string()
}

fn with_collision_suffix(path: &Path) -> PathBuf {
    let suffix = Uuid::new_v4().simple().to_string();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let ext = path.extension().and_then(|value| value.to_str());

    match ext {
        Some(ext) if !ext.is_empty() => path.with_file_name(format!("{stem}-{suffix}.{ext}")),
        _ => path.with_file_name(format!("{stem}-{suffix}")),
    }
}

/// Sanitizes filenames to prevent traversal and unsafe path characters.
pub fn sanitize_filename(input: &str) -> String {
    let base = Path::new(input)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");

    let mut sanitized: String = base
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    sanitized = sanitized.trim_matches(['.', ' ']).to_owned();
    if sanitized.is_empty() || sanitized == "." || sanitized == ".." {
        return "file".to_owned();
    }

    sanitized
}



