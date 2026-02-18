use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::{StorageEngine, StoredFile};
use crate::{Part, StorageError};

type CustomFilenameFn = dyn Fn(String) -> String + Send + Sync;

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
#[derive(Debug, Clone)]
pub struct DiskStorageBuilder {
    root: PathBuf,
    strategy: FilenameStrategy,
}

impl DiskStorageBuilder {
    /// Sets the directory used for persisted files.
    pub fn path(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = root.into();
        self
    }

    /// Sets how output filenames are generated.
    pub fn filename_strategy(mut self, strategy: FilenameStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Sets a custom filename function.
    pub fn custom_filename<F>(mut self, transform: F) -> Self
    where
        F: Fn(String) -> String + Send + Sync + 'static,
    {
        self.strategy = FilenameStrategy::Custom(Arc::new(transform));
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
        })
    }
}

impl Default for DiskStorageBuilder {
    fn default() -> Self {
        Self {
            root: std::env::temp_dir().join("rust-multer"),
            strategy: FilenameStrategy::Random,
        }
    }
}

/// Disk-backed storage engine writing files under a configured root path.
#[derive(Debug, Clone)]
pub struct DiskStorage {
    root: PathBuf,
    strategy: FilenameStrategy,
}

impl DiskStorage {
    /// Creates a disk storage builder.
    pub fn builder() -> DiskStorageBuilder {
        DiskStorageBuilder::default()
    }

    fn choose_output_name(&self, part: &Part) -> String {
        let input_name = part
            .file_name()
            .map(ToOwned::to_owned)
            .unwrap_or_else(random_basename);

        let candidate = match &self.strategy {
            FilenameStrategy::Keep => input_name,
            FilenameStrategy::Random => random_basename(),
            FilenameStrategy::Custom(transform) => transform(input_name),
        };

        sanitize_filename(&candidate)
    }
}

#[async_trait::async_trait]
impl StorageEngine for DiskStorage {
    async fn store(&self, mut part: Part) -> Result<StoredFile, StorageError> {
        tokio::fs::create_dir_all(&self.root)
            .await
            .map_err(|err| StorageError::new(format!("failed to create storage directory: {err}")))?;

        let field_name = part.field_name().to_owned();
        let file_name = part.file_name().map(ToOwned::to_owned);
        let content_type = part.content_type().clone();
        let file_basename = self.choose_output_name(&part);

        let mut output_path = self.root.join(file_basename);
        if tokio::fs::try_exists(&output_path)
            .await
            .map_err(|err| StorageError::new(format!("failed to inspect output path: {err}")))?
        {
            output_path = with_collision_suffix(&output_path);
        }

        let mut file = tokio::fs::File::create(&output_path)
            .await
            .map_err(|err| StorageError::new(format!("failed to create output file: {err}")))?;

        let mut stream = part
            .stream()
            .map_err(|err| StorageError::new(format!("failed to read part stream: {err}")))?;
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
        Ok(StoredFile {
            storage_key,
            field_name,
            file_name,
            content_type,
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
