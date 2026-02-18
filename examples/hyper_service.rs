#![allow(missing_docs)]

#[cfg(feature = "hyper")]
use std::sync::Arc;

#[cfg(feature = "hyper")]
use bytes::Bytes;
#[cfg(feature = "hyper")]
use http_body_util::Full;
#[cfg(feature = "hyper")]
use hyper::{header, service::Service, Request, Response};
#[cfg(feature = "hyper")]
use rust_multer::{hyper::MulterService, DiskStorage, FilenameStrategy, Multer, StoredFile};

#[cfg(feature = "hyper")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("rust-multer-hyper-service"))
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

#[cfg(not(feature = "hyper"))]
fn main() {
    println!("Enable the `hyper` feature to run this example.");
}
