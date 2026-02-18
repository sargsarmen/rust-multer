#![allow(missing_docs)]

#[cfg(feature = "hyper")]
use std::sync::Arc;

#[cfg(feature = "hyper")]
use bytes::Bytes;
#[cfg(feature = "hyper")]
use http_body_util::{BodyExt, Full};
#[cfg(feature = "hyper")]
use hyper::{header, service::Service, Request, Response};
#[cfg(feature = "hyper")]
use multigear::{extract_boundary, hyper::MulterService, MemoryStorage, Multer, StoredFile};

#[cfg(feature = "hyper")]
#[tokio::test]
async fn parse_stream_accepts_hyper_data_stream() {
    let multer = Multer::new(MemoryStorage::new());
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "value\r\n",
        "--BOUND--\r\n"
    );
    let request = Request::builder()
        .header(header::CONTENT_TYPE, "multipart/form-data; boundary=BOUND")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("request should build");

    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .expect("content type should exist");
    let boundary = extract_boundary(content_type).expect("boundary should parse");
    let stream = request.into_body().into_data_stream();

    let mut multipart = multer
        .parse_stream(stream, boundary)
        .await
        .expect("multipart should initialize");
    let mut part = multipart
        .next_part()
        .await
        .expect("part parse should succeed")
        .expect("part should exist");

    assert_eq!(part.field_name(), "field");
    assert_eq!(part.text().await.expect("text should decode"), "value");
}

#[cfg(feature = "hyper")]
#[tokio::test]
async fn multer_service_stores_file_parts_and_calls_handler() {
    let multer = Arc::new(Multer::new(MemoryStorage::new()));
    let service = MulterService::new(multer, |saved_files: Vec<StoredFile>| async move {
        Ok::<_, std::io::Error>(Response::new(Full::new(Bytes::from(format!(
            "{}",
            saved_files.len()
        )))))
    });

    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "hello\r\n",
        "--BOUND--\r\n"
    );
    let request = Request::builder()
        .header(header::CONTENT_TYPE, "multipart/form-data; boundary=BOUND")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("request should build");

    let response = service.call(request).await.expect("service should succeed");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    assert_eq!(body.as_ref(), b"1");
}

#[cfg(feature = "hyper")]
#[tokio::test]
async fn multer_service_rejects_requests_without_content_type() {
    let multer = Arc::new(Multer::new(MemoryStorage::new()));
    let service = MulterService::new(multer, |_saved_files: Vec<StoredFile>| async move {
        Ok::<_, std::io::Error>(Response::new(Full::new(Bytes::from_static(b"ok"))))
    });

    let request = Request::builder()
        .body(Full::new(Bytes::from_static(b"no header")))
        .expect("request should build");

    let err = service
        .call(request)
        .await
        .expect_err("service should fail");
    assert!(err.to_string().contains("missing Content-Type"));
}

