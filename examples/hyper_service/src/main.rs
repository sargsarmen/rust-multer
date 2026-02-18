#![allow(missing_docs)]

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, service::Service, Request, Response};
use multigear::{hyper::MulterService, DiskStorage, FilenameStrategy, Multer, StoredFile};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-hyper-service"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Arc::new(
        Multer::builder()
            .any()
            .storage(storage)
            .build()
            .expect("multer should build"),
    );

    let service = MulterService::new(multer, |saved_files: Vec<StoredFile>| async move {
        Ok::<_, std::io::Error>(Response::new(Full::new(Bytes::from(format!(
            "stored {} file(s)",
            saved_files.len()
        )))))
    });

    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "hello from MulterService\r\n",
        "--BOUND--\r\n"
    );

    let request = Request::builder()
        .header(header::CONTENT_TYPE, "multipart/form-data; boundary=BOUND")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("request should build");

    let response = service.call(request).await.expect("service should succeed");
    println!("hyper service status: {}", response.status());
}


