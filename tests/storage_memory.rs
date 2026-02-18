#![allow(missing_docs)]

use bytes::Bytes;
use futures::stream;
use multigear::{MemoryStorage, Multer, MulterError, Multipart};

#[tokio::test]
async fn stores_file_part_and_returns_metadata() {
    let storage = MemoryStorage::new();
    let multer = Multer::new(storage.clone());

    let body = multipart_body(&[("avatar", "face.png", "image/png", "hello")]);
    let mut multipart =
        Multipart::new("BOUND", bytes_stream(body)).expect("multipart should initialize");
    let part = multipart
        .next_part().await.expect("part should parse").expect("part expected");

    let stored = multer.store(part).await.expect("store should succeed");
    assert_eq!(stored.field_name, "avatar");
    assert_eq!(stored.file_name.as_deref(), Some("face.png"));
    assert_eq!(stored.content_type.essence_str(), "image/png");
    assert_eq!(stored.size, 5);

    let payload = storage
        .get(&stored.storage_key)
        .await
        .expect("payload should exist");
    assert_eq!(payload, Bytes::from_static(b"hello"));
}

#[tokio::test]
async fn memory_storage_conformance_unique_keys_and_payload_integrity() {
    let storage = MemoryStorage::new();
    let multer = Multer::new(storage.clone());

    let body = multipart_body(&[
        ("a", "a.bin", "application/octet-stream", "one"),
        ("b", "b.bin", "application/octet-stream", "two"),
    ]);
    let mut multipart =
        Multipart::new("BOUND", bytes_stream(body)).expect("multipart should initialize");

    let first = multipart
        .next_part().await.expect("first part should parse").expect("first part expected");
    let first_meta = multer.store(first).await.expect("first store should succeed");
    let second = multipart
        .next_part().await.expect("second part should parse").expect("second part expected");
    let second_meta = multer.store(second).await.expect("second store should succeed");

    assert_ne!(first_meta.storage_key, second_meta.storage_key);
    assert_eq!(storage.len().await, 2);
    assert_eq!(
        storage.get(&first_meta.storage_key).await,
        Some(Bytes::from_static(b"one"))
    );
    assert_eq!(
        storage.get(&second_meta.storage_key).await,
        Some(Bytes::from_static(b"two"))
    );
}

fn multipart_body(parts: &[(&str, &str, &str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (field, file_name, content_type, body) in parts {
        out.extend_from_slice(b"--BOUND\r\n");
        let disposition =
            format!("Content-Disposition: form-data; name=\"{field}\"; filename=\"{file_name}\"\r\n");
        out.extend_from_slice(disposition.as_bytes());
        let content_type = format!("Content-Type: {content_type}\r\n\r\n");
        out.extend_from_slice(content_type.as_bytes());
        out.extend_from_slice(body.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"--BOUND--\r\n");
    out
}

fn bytes_stream(body: Vec<u8>) -> impl futures::Stream<Item = Result<Bytes, MulterError>> {
    stream::iter([Ok(Bytes::from(body))])
}



