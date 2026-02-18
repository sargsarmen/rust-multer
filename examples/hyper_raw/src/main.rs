#![allow(missing_docs)]

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{header, Request};
use multigear::{DiskStorage, FilenameStrategy, Multer};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-hyper-raw"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Multer::builder()
        .any()
        .storage(storage)
        .build()
        .expect("multer should build");

    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "hello from hyper\r\n",
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
    let boundary = multigear::extract_boundary(content_type).expect("boundary should parse");

    let stream = request.into_body().into_data_stream();
    let mut multipart = multer
        .parse_stream(stream, boundary)
        .await
        .expect("multipart should parse");

    let mut stored = 0usize;
    while let Some(part) = multipart
        .next_part()
        .await
        .expect("part parse should succeed")
    {
        if part.file_name().is_some() {
            let _stored_file = multer.store(part).await.expect("storage should succeed");
            stored += 1;
        }
    }

    println!("stored {stored} file(s) via hyper Level 1 bridge");
}


