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
pub use error::MulterError;
pub use field::{Field, FieldKind, FileField, TextField};
pub use limits::Limits;
pub use multipart::Multipart;
pub use part::Part;
pub use storage::{NoopStorage, StorageEngine};

/// Main `rust-multer` entry point.
#[derive(Debug)]
pub struct Multer<S = NoopStorage> {
    storage: S,
}

impl<S> Multer<S> {
    /// Creates a new multer instance with the given storage backend.
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    /// Returns an immutable reference to the configured storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }
}
