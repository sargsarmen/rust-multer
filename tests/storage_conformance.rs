#![allow(missing_docs)]

use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use futures::{StreamExt, stream};
use rust_multer::{BoxStream, Multer, MulterError, Multipart, StorageEngine, StorageError};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
struct MapStorage {
    items: Arc<RwLock<HashMap<String, Bytes>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MapStoredFile {
    key: String,
    field_name: String,
    file_name: Option<String>,
    content_type: String,
    size: u64,
}

#[async_trait::async_trait]
impl StorageEngine for MapStorage {
    type Output = MapStoredFile;
    type Error = StorageError;

    async fn store(
        &self,
        field_name: &str,
        file_name: Option<&str>,
        content_type: &str,
        mut stream: BoxStream<'_, Result<Bytes, MulterError>>,
    ) -> Result<Self::Output, Self::Error> {
        let key = format!("{field_name}-{}", self.items.read().await.len());
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|err| StorageError::new(err.to_string()))?;
            bytes.extend_from_slice(&chunk);
        }
        let bytes = Bytes::from(bytes);
        let size = bytes.len() as u64;

        self.items.write().await.insert(key.clone(), bytes);
        Ok(MapStoredFile {
            key,
            field_name: field_name.to_owned(),
            file_name: file_name.map(ToOwned::to_owned),
            content_type: content_type.to_owned(),
            size,
        })
    }
}

#[tokio::test]
async fn custom_storage_backend_conforms_to_store_contract() {
    let storage = MapStorage::default();
    let multer = Multer::new(storage.clone());

    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"doc\"; filename=\"a.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "hello\r\n",
        "--BOUND--\r\n"
    );
    let mut multipart = Multipart::new(
        "BOUND",
        stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]),
    )
    .expect("multipart should initialize");

    let part = multipart
        .next_part().await.expect("part should parse").expect("part expected");
    let stored = multer.store(part).await.expect("store should succeed");

    assert_eq!(stored.field_name, "doc");
    assert_eq!(stored.size, 5);
    assert_eq!(
        storage.items.read().await.get(&stored.key).cloned(),
        Some(Bytes::from_static(b"hello"))
    );
}

#[tokio::test]
async fn custom_storage_works_with_parse_and_store_pipeline() {
    let storage = MapStorage::default();
    let multer = Multer::new(storage.clone());
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"a.bin\"\r\n",
        "\r\n",
        "one\r\n",
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"note\"\r\n",
        "\r\n",
        "two\r\n",
        "--BOUND--\r\n"
    );

    let output = multer
        .parse_and_store(
            "BOUND",
            stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]),
        )
        .await
        .expect("pipeline should succeed");

    assert_eq!(output.stored_files.len(), 1);
    assert_eq!(output.text_fields, vec![("note".to_owned(), "two".to_owned())]);
}



