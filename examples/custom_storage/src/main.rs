#![allow(missing_docs)]

use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use futures::{StreamExt, stream};
use multigear::{BoxStream, Multer, MulterError, StorageEngine, StorageError};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
struct HashMapStorage {
    files: Arc<RwLock<HashMap<String, Bytes>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HashMapKey(String);

#[async_trait::async_trait]
impl StorageEngine for HashMapStorage {
    type Output = HashMapKey;
    type Error = StorageError;

    async fn store(
        &self,
        field_name: &str,
        _file_name: Option<&str>,
        _content_type: &str,
        mut stream: BoxStream<'_, Result<Bytes, MulterError>>,
    ) -> Result<Self::Output, Self::Error> {
        let key = format!("{field_name}-{}", self.files.read().await.len());
        let mut content = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|err| StorageError::new(err.to_string()))?;
            content.extend_from_slice(&chunk);
        }

        self.files
            .write()
            .await
            .insert(key.clone(), Bytes::from(content));
        Ok(HashMapKey(key))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let storage = HashMapStorage::default();
    let multer = Multer::new(storage.clone());

    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"upload\"; filename=\"a.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "hello world\r\n",
        "--BOUND--\r\n"
    );
    let output = multer
        .parse_and_store(
            "BOUND",
            stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]),
        )
        .await
        .expect("pipeline should succeed");

    println!("stored files: {}", output.stored_files.len());
}

