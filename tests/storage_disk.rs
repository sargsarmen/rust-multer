#![allow(missing_docs)]

use std::path::PathBuf;

use bytes::Bytes;
use futures::{StreamExt, stream};
use rust_multer::{DiskStorage, FilenameStrategy, Multer, MulterError, Multipart};
use uuid::Uuid;

#[tokio::test]
async fn keep_strategy_sanitizes_filename_and_writes_to_disk() {
    let root = temp_root();
    let storage = DiskStorage::builder()
        .path(&root)
        .filename_strategy(FilenameStrategy::Keep)
        .build()
        .expect("builder should succeed");
    let multer = Multer::new(storage);

    let body = multipart_body(&[("upload", "..\\..\\bad:name?.txt", "text/plain", "hello")]);
    let mut multipart =
        Multipart::new("BOUND", bytes_stream(body)).expect("multipart should initialize");
    let part = multipart
        .next()
        .await
        .expect("part expected")
        .expect("part should parse");

    let stored = multer.store(part).await.expect("store should succeed");
    let path = stored.path.clone().expect("disk storage should return a path");
    assert!(path.starts_with(&root));
    assert_eq!(stored.size, 5);
    assert_eq!(tokio::fs::read(&path).await.expect("read file"), b"hello");

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .expect("valid filename");
    assert!(!file_name.contains(".."));
    assert!(!file_name.contains(':'));

    cleanup(root).await;
}

#[tokio::test]
async fn random_strategy_generates_distinct_paths() {
    let root = temp_root();
    let storage = DiskStorage::builder()
        .path(&root)
        .filename_strategy(FilenameStrategy::Random)
        .build()
        .expect("builder should succeed");
    let multer = Multer::new(storage);

    let body = multipart_body(&[
        ("a", "same.txt", "text/plain", "one"),
        ("b", "same.txt", "text/plain", "two"),
    ]);
    let mut multipart =
        Multipart::new("BOUND", bytes_stream(body)).expect("multipart should initialize");

    let first = multipart
        .next()
        .await
        .expect("first expected")
        .expect("first should parse");
    let second = multipart
        .next()
        .await
        .expect("second expected")
        .expect("second should parse");

    let first_stored = multer.store(first).await.expect("first store");
    let second_stored = multer.store(second).await.expect("second store");
    assert_ne!(first_stored.path, second_stored.path);

    cleanup(root).await;
}

#[tokio::test]
async fn custom_strategy_applies_transform() {
    let root = temp_root();
    let storage = DiskStorage::builder()
        .path(&root)
        .custom_filename(|incoming| format!("prefix-{incoming}"))
        .build()
        .expect("builder should succeed");
    let multer = Multer::new(storage);

    let body = multipart_body(&[("doc", "report.txt", "text/plain", "payload")]);
    let mut multipart =
        Multipart::new("BOUND", bytes_stream(body)).expect("multipart should initialize");
    let part = multipart
        .next()
        .await
        .expect("part expected")
        .expect("part should parse");

    let stored = multer.store(part).await.expect("store should succeed");
    let file_name = stored
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
        .expect("valid filename");
    assert!(file_name.starts_with("prefix-report"));

    cleanup(root).await;
}

fn temp_root() -> PathBuf {
    std::env::temp_dir().join(format!("rust-multer-test-{}", Uuid::new_v4()))
}

async fn cleanup(path: PathBuf) {
    let _ = tokio::fs::remove_dir_all(path).await;
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
