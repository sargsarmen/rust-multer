use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use futures::StreamExt;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{BoxStream, StorageEngine, StoredFile};
use crate::{MulterError, StorageError};

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

    /// Returns `true` when no payloads are currently stored.
    pub async fn is_empty(&self) -> bool {
        self.files.read().await.is_empty()
    }
}

#[async_trait::async_trait(?Send)]
impl StorageEngine for MemoryStorage {
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
            "memory storage: begin streaming store"
        );

        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|err| StorageError::new(err.to_string()))?;
            body.extend_from_slice(&chunk);
        }
        let body = Bytes::from(body);

        let storage_key = Uuid::new_v4().to_string();
        let size = body.len() as u64;
        let parsed_content_type = content_type
            .parse::<mime::Mime>()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM);

        self.files.write().await.insert(storage_key.clone(), body);

        #[cfg(feature = "tracing")]
        tracing::debug!(
            field_name = field_name,
            storage_key = storage_key.as_str(),
            size = size,
            "memory storage: completed store"
        );

        Ok(StoredFile {
            storage_key,
            field_name: field_name.to_owned(),
            file_name: file_name.map(ToOwned::to_owned),
            content_type: parsed_content_type,
            size,
            path: None,
        })
    }
}


