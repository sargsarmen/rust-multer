#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! Core crate surface for `rust-multer`.

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
}

impl Multer<NoopStorage> {
    /// Creates a fluent builder with permissive defaults.
    pub fn builder() -> MulterBuilder {
        MulterBuilder::default()
    }
}
