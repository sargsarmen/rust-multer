#![allow(missing_docs)]

use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use futures::{StreamExt, stream};
use rust_multer::{Multer, MulterError, Multipart, StorageEngine, StorageError, StoredFile};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
struct MapStorage {
    items: Arc<RwLock<HashMap<String, Bytes>>>,
}

#[async_trait::async_trait]
impl StorageEngine for MapStorage {
    async fn store(&self, mut part: rust_multer::Part) -> Result<StoredFile, StorageError> {
        let key = format!("{}-{}", part.field_name(), self.items.read().await.len());
        let bytes = part
            .bytes()
            .await
            .map_err(|err| StorageError::new(err.to_string()))?;
        let size = bytes.len() as u64;
        let field_name = part.field_name().to_owned();
        let file_name = part.file_name().map(ToOwned::to_owned);
        let content_type = part.content_type().clone();

        self.items.write().await.insert(key.clone(), bytes);
        Ok(StoredFile {
            storage_key: key,
            field_name,
            file_name,
            content_type,
            size,
            path: None,
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
        .next()
        .await
        .expect("part expected")
        .expect("part should parse");
    let stored = multer.store(part).await.expect("store should succeed");

    assert_eq!(stored.field_name, "doc");
    assert_eq!(stored.size, 5);
    assert_eq!(
        storage.items.read().await.get(&stored.storage_key).cloned(),
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
