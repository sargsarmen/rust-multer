#![allow(missing_docs)]

use bytes::Bytes;
use futures::{TryStreamExt, stream};
use multigear::{Multipart, MulterError, ParseError};

#[tokio::test]
async fn exposes_metadata_accessors() {
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"avatar\"; filename=\"face.png\"\r\n",
        "Content-Type: image/png\r\n",
        "Content-Length: 3\r\n",
        "\r\n",
        "abc\r\n",
        "--BOUND--\r\n"
    );

    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");
    let part = multipart
        .next_part()
        .await
        .expect("part expected")
        .expect("part should parse");

    assert_eq!(part.field_name(), "avatar");
    assert_eq!(part.file_name(), Some("face.png"));
    assert_eq!(part.content_type(), "image/png");
    assert_eq!(
        part.headers()
            .get("content-disposition")
            .and_then(|value| value.to_str().ok()),
        Some("form-data; name=\"avatar\"; filename=\"face.png\"")
    );
    assert_eq!(part.parsed_headers().field_name, "avatar");
    assert_eq!(part.size_hint(), Some(3));
}

#[tokio::test]
async fn bytes_are_single_pass() {
    let input_body =
        "--BOUND\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\nhello\r\n--BOUND--\r\n";
    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(
        input_body.as_bytes(),
    ))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");
    let mut part = multipart
        .next_part()
        .await
        .expect("part expected")
        .expect("part should parse");
    assert_eq!(part.size_hint(), None);

    let payload = part.bytes().await.expect("bytes should be readable");
    assert_eq!(payload, Bytes::from_static(b"hello"));
    assert_eq!(part.size_hint(), None);

    let err = part.bytes().await.expect_err("second read must fail");
    assert_already_consumed(err);
}

#[tokio::test]
async fn stream_is_single_pass_and_returns_body() {
    let input_body = "--BOUND\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\nstream-body\r\n--BOUND--\r\n";
    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from(
        input_body.as_bytes().to_vec(),
    ))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");
    let mut part = multipart
        .next_part()
        .await
        .expect("part expected")
        .expect("part should parse");

    let stream = part.stream();
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
        .next_part()
        .await
        .expect("part expected")
        .expect("part should parse");

    let err = part.text().await.expect_err("invalid UTF-8 should fail");
    assert!(matches!(
        err,
        MulterError::Parse(ParseError::Message { .. })
    ));
}

fn assert_already_consumed(err: MulterError) {
    assert!(
        err.to_string().contains("already consumed"),
        "unexpected error: {err}"
    );
}


