use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{StorageEngine, StoredFile};
use crate::{Part, StorageError};

/// In-memory storage engine keyed by generated UUIDs.
#[derive(Debug, Clone, Default)]
pub struct MemoryStorage {
    files: Arc<RwLock<HashMap<String, Bytes>>>,
}

impl MemoryStorage {
    /// Creates an empty in-memory storage backend.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns stored bytes for a previously stored key.
    pub async fn get(&self, key: &str) -> Option<Bytes> {
        self.files.read().await.get(key).cloned()
    }

    /// Returns the current number of stored objects.
    pub async fn len(&self) -> usize {
        self.files.read().await.len()
    }
}

#[async_trait::async_trait]
impl StorageEngine for MemoryStorage {
    async fn store(&self, mut part: Part) -> Result<StoredFile, StorageError> {
        let field_name = part.field_name().to_owned();
        let file_name = part.file_name().map(ToOwned::to_owned);
        let content_type = part.content_type().clone();
        let body = part
            .bytes()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;

        let storage_key = Uuid::new_v4().to_string();
        let size = body.len() as u64;

        self.files.write().await.insert(storage_key.clone(), body);

        Ok(StoredFile {
            storage_key,
            field_name,
            file_name,
            content_type,
            size,
            path: None,
        })
    }
}
