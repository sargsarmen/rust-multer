#![allow(missing_docs)]

use bytes::Bytes;
use futures::stream;
use rust_multer::{
    Limits, MemoryStorage, Multer, MulterConfig, MulterError, Selector, UnknownFieldPolicy,
};

#[tokio::test]
async fn parse_and_store_wires_parser_selector_limits_and_storage() {
    let storage = MemoryStorage::new();
    let config = MulterConfig {
        selector: Selector::single("avatar"),
        unknown_field_policy: UnknownFieldPolicy::Reject,
        limits: Limits {
            max_files: Some(1),
            max_fields: Some(1),
            allowed_mime_types: vec!["image/*".to_owned()],
            ..Limits::default()
        },
    };
    let multer = Multer::with_config(storage.clone(), config).expect("config should validate");

    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"avatar\"; filename=\"a.png\"\r\n",
        "Content-Type: image/png\r\n",
        "\r\n",
        "PNGDATA\r\n",
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"note\"\r\n",
        "\r\n",
        "hello\r\n",
        "--BOUND--\r\n"
    );

    let output = multer
        .parse_and_store("BOUND", stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]))
        .await
        .expect("pipeline should succeed");

    assert_eq!(output.stored_files.len(), 1);
    assert_eq!(output.text_fields, vec![("note".to_owned(), "hello".to_owned())]);

    let stored = &output.stored_files[0];
    let bytes = storage
        .get(&stored.storage_key)
        .await
        .expect("stored payload should exist");
    assert_eq!(bytes, Bytes::from_static(b"PNGDATA"));
}

#[tokio::test]
async fn multipart_from_content_type_is_framework_agnostic_entry_point() {
    let multer = Multer::new(MemoryStorage::new());
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "value\r\n",
        "--BOUND--\r\n"
    );

    let mut multipart = multer
        .multipart_from_content_type(
            "multipart/form-data; boundary=BOUND",
            stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]),
        )
        .expect("content type should parse");

    let part = futures::StreamExt::next(&mut multipart)
        .await
        .expect("part expected")
        .expect("part should parse");
    assert_eq!(part.field_name(), "field");
}

#[tokio::test]
async fn parse_and_store_reports_malformed_stream_regression() {
    let multer = Multer::new(MemoryStorage::new());
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"a.bin\"\r\n",
        "\r\n",
        "payload"
    );

    let result = multer
        .parse_and_store("BOUND", stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]))
        .await;
    assert!(matches!(result, Err(MulterError::IncompleteStream)));
}

#[tokio::test]
async fn parse_and_store_respects_unknown_field_policy_regression() {
    let config = MulterConfig {
        selector: Selector::single("avatar"),
        unknown_field_policy: UnknownFieldPolicy::Reject,
        ..MulterConfig::default()
    };
    let multer = Multer::with_config(MemoryStorage::new(), config).expect("config should validate");
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"other\"; filename=\"x.bin\"\r\n",
        "\r\n",
        "hello\r\n",
        "--BOUND--\r\n"
    );

    let result = multer
        .parse_and_store("BOUND", stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]))
        .await;
    assert!(matches!(
        result,
        Err(MulterError::UnexpectedField { field }) if field == "other"
    ));
}
