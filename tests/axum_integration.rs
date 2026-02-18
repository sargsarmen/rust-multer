#![allow(missing_docs)]

#[cfg(feature = "axum")]
use std::sync::Arc;

#[cfg(feature = "axum")]
use axum::{
    body::Body,
    extract::FromRequest,
    http::{Request, header},
};
#[cfg(feature = "axum")]
use rust_multer::{MemoryStorage, Multer, axum::MulterExtractor};

#[cfg(feature = "axum")]
#[tokio::test]
async fn multer_extractor_parses_axum_request_body() {
    let state = Arc::new(Multer::new(MemoryStorage::new()));
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "value\r\n",
        "--BOUND--\r\n"
    );
    let request = Request::builder()
        .header(header::CONTENT_TYPE, "multipart/form-data; boundary=BOUND")
        .body(Body::from(body))
        .expect("request should build");

    let MulterExtractor(mut multipart) = MulterExtractor::from_request(request, &state)
        .await
        .expect("extractor should parse multipart");
    let mut part = multipart
        .next_part()
        .await
        .expect("part parsing should succeed")
        .expect("part should exist");

    assert_eq!(part.field_name(), "field");
    assert_eq!(part.text().await.expect("text body should decode"), "value");
}

