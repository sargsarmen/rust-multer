#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! Core crate surface for `rust-multer`.

use bytes::Bytes;
use futures::{Stream, StreamExt};

/// Fluent builder API.
pub mod builder;
/// Multipart parser configuration.
pub mod config;
/// Error types exposed by this crate.
pub mod error;
/// Field selection and matching models.
pub mod field;
/// Request and field limits.
pub mod limits;
/// High-level multipart stream type.
pub mod multipart;
/// Parsed multipart part API.
pub mod part;
/// Runtime selector engine.
pub mod selector;
/// Low-level parser components.
pub mod parser;
/// Storage engine traits and implementations.
pub mod storage;

#[cfg(feature = "actix")]
pub mod actix;
#[cfg(feature = "axum")]
pub mod axum;

pub use builder::MulterBuilder;
pub use config::{MulterConfig, SelectedField, Selector, UnknownFieldPolicy};
pub use error::{ConfigError, MulterError, ParseError, StorageError};
pub use field::{Field, FieldKind, FileField, TextField};
pub use limits::Limits;
pub use multipart::Multipart;
pub use part::Part;
pub use selector::{SelectorAction, SelectorEngine};
pub use storage::{
    DiskStorage, DiskStorageBuilder, FilenameStrategy, MemoryStorage, NoopStorage, StorageEngine,
    StoredFile,
};

/// Processed multipart output returned by [`Multer::parse_and_store`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProcessedMultipart {
    /// File parts persisted through the configured storage engine.
    pub stored_files: Vec<StoredFile>,
    /// Text field values collected from the stream.
    pub text_fields: Vec<(String, String)>,
}

/// Main `rust-multer` entry point.
#[derive(Debug)]
pub struct Multer<S = NoopStorage> {
    config: MulterConfig,
    storage: S,
}

impl<S> Multer<S> {
    /// Creates a new multer instance with the given storage backend.
    pub fn new(storage: S) -> Self {
        Self {
            config: MulterConfig::default(),
            storage,
        }
    }

    /// Creates a new multer instance with explicit validated configuration.
    pub fn with_config(storage: S, config: MulterConfig) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self { config, storage })
    }

    /// Returns an immutable reference to the active configuration.
    pub fn config(&self) -> &MulterConfig {
        &self.config
    }

    /// Returns an immutable reference to the configured storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

impl<S> Multer<S>
where
    S: StorageEngine,
{
    /// Stores a file part through the configured storage backend.
    pub async fn store(&self, part: Part) -> Result<StoredFile, MulterError> {
        self.storage.store(part).await.map_err(MulterError::from)
    }

    /// Creates a configured multipart parser from a raw multipart boundary.
    pub fn multipart_from_boundary<T>(
        &self,
        boundary: impl Into<String>,
        stream: T,
    ) -> Result<Multipart<T>, MulterError>
    where
        T: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        Multipart::with_config(boundary, stream, self.config.clone())
    }

    /// Creates a configured multipart parser from an HTTP `Content-Type` value.
    pub fn multipart_from_content_type<T>(
        &self,
        content_type: &str,
        stream: T,
    ) -> Result<Multipart<T>, MulterError>
    where
        T: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        let boundary = parser::extract_multipart_boundary(content_type)?;
        self.multipart_from_boundary(boundary, stream)
    }

    /// Parses multipart input and stores all file parts using the active storage backend.
    pub async fn parse_and_store<T>(
        &self,
        boundary: impl Into<String>,
        stream: T,
    ) -> Result<ProcessedMultipart, MulterError>
    where
        T: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        let mut multipart = self.multipart_from_boundary(boundary, stream)?;
        let mut out = ProcessedMultipart::default();

        while let Some(item) = multipart.next().await {
            let mut part = item?;
            if part.file_name().is_some() {
                let stored = self.store(part).await?;
                out.stored_files.push(stored);
            } else {
                let field_name = part.field_name().to_owned();
                let text = part.text().await?;
                out.text_fields.push((field_name, text));
            }
        }

        Ok(out)
    }
}

impl Multer<NoopStorage> {
    /// Creates a fluent builder with permissive defaults.
    pub fn builder() -> MulterBuilder {
        MulterBuilder::default()
    }
}
