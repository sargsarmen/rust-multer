//! Storage engine abstractions and built-in implementations.

use crate::{Part, StorageError};

/// Disk-backed storage backend implementation.
pub mod disk;
/// In-memory storage backend implementation.
pub mod memory;
pub use disk::{DiskStorage, DiskStorageBuilder, FilenameStrategy};
pub use memory::MemoryStorage;

/// Metadata describing a stored file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFile {
    /// Backend-specific opaque key or location identifier.
    pub storage_key: String,
    /// Multipart field name.
    pub field_name: String,
    /// Original filename from the multipart part, when present.
    pub file_name: Option<String>,
    /// Content type observed on the uploaded file part.
    pub content_type: mime::Mime,
    /// Persisted file size in bytes.
    pub size: u64,
    /// Final filesystem path when stored on disk.
    pub path: Option<std::path::PathBuf>,
}

/// Async trait abstraction for file storage backends.
#[async_trait::async_trait]
pub trait StorageEngine: Send + Sync + std::fmt::Debug {
    /// Stores a file part and returns output metadata.
    async fn store(&self, part: Part) -> Result<StoredFile, StorageError>;
}

/// Placeholder storage implementation used as the default backend.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopStorage;

#[async_trait::async_trait]
impl StorageEngine for NoopStorage {
    async fn store(&self, _part: Part) -> Result<StoredFile, StorageError> {
        Err(StorageError::new(
            "no storage backend configured; choose a concrete storage engine",
        ))
    }
}
