#![allow(missing_docs)]

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt, stream};
use rust_multer::{Multipart, MulterError, ParseError};

#[tokio::test]
async fn exposes_metadata_accessors() {
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"avatar\"; filename=\"face.png\"\r\n",
        "Content-Type: image/png\r\n",
        "\r\n",
        "abc\r\n",
        "--BOUND--\r\n"
    );

    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");
    let part = multipart
        .next()
        .await
        .expect("part expected")
        .expect("part should parse");

    assert_eq!(part.field_name(), "avatar");
    assert_eq!(part.file_name(), Some("face.png"));
    assert_eq!(part.content_type().essence_str(), "image/png");
    assert_eq!(part.headers().field_name, "avatar");
}

#[tokio::test]
async fn bytes_are_single_pass() {
    let mut part = parse_single_part_body("hello").await;
    assert_eq!(part.size_hint(), 5);

    let payload = part.bytes().await.expect("bytes should be readable");
    assert_eq!(payload, Bytes::from_static(b"hello"));
    assert_eq!(part.size_hint(), 0);

    let err = part.bytes().await.expect_err("second read must fail");
    assert_already_consumed(err);
}

#[tokio::test]
async fn stream_is_single_pass_and_returns_body() {
    let mut part = parse_single_part_body("stream-body").await;

    let stream = part.stream().expect("stream should be created");
    let chunks = stream.try_collect::<Vec<_>>().await.expect("stream should read");
    assert_eq!(chunks, vec![Bytes::from_static(b"stream-body")]);

    let err = part.text().await.expect_err("second read must fail");
    assert_already_consumed(err);
}

#[tokio::test]
async fn text_rejects_non_utf8_payloads() {
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"note\"\r\n",
        "\r\n",
    );
    let invalid = [0x66u8, 0x6f, 0x80];
    let trailer = concat!("\r\n", "--BOUND--\r\n");

    let mut bytes = Vec::new();
    bytes.extend_from_slice(body.as_bytes());
    bytes.extend_from_slice(&invalid);
    bytes.extend_from_slice(trailer.as_bytes());

    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from(bytes))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");
    let mut part = multipart
        .next()
        .await
        .expect("part expected")
        .expect("part should parse");

    let err = part.text().await.expect_err("invalid UTF-8 should fail");
    assert!(matches!(
        err,
        MulterError::Parse(ParseError::Message { .. })
    ));
}

async fn parse_single_part_body(body: &str) -> rust_multer::Part {
    let input_body = format!(
        "--BOUND\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\n{body}\r\n--BOUND--\r\n"
    );

    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from(input_body.into_bytes()))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");
    multipart
        .next()
        .await
        .expect("part expected")
        .expect("part should parse")
}

fn assert_already_consumed(err: MulterError) {
    assert!(
        err.to_string().contains("already consumed"),
        "unexpected error: {err}"
    );
}
