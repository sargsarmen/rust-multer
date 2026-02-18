#![allow(missing_docs)]

use bytes::Bytes;
use futures::{StreamExt, channel::mpsc, stream};
use rust_multer::{Multipart, MulterError, ParseError};

#[tokio::test]
async fn parses_chunked_stream_and_yields_parts() {
    let body = concat!(
        "--XBOUND\r\n",
        "Content-Disposition: form-data; name=\"alpha\"\r\n",
        "\r\n",
        "one\r\n",
        "--XBOUND\r\n",
        "Content-Disposition: form-data; name=\"beta\"; filename=\"b.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "two\r\n",
        "--XBOUND--\r\n"
    );

    let chunks = split_bytes(body.as_bytes(), &[3, 2, 7, 1, 4, 9, 5, 8, 6, 64]);
    let stream = stream::iter(chunks.into_iter().map(Ok::<Bytes, MulterError>));
    let mut multipart = Multipart::new("XBOUND", stream).expect("boundary should be valid");

    let mut first = multipart
        .next_part()
        .await
        .expect("first part should parse")
        .expect("first item should exist");
    assert_eq!(first.headers.field_name, "alpha");
    assert!(first.headers.file_name.is_none());
    assert_eq!(first.bytes().await.expect("body bytes"), Bytes::from_static(b"one"));

    let mut second = multipart
        .next_part()
        .await
        .expect("second part should parse")
        .expect("second item should exist");
    assert_eq!(second.headers.field_name, "beta");
    assert_eq!(second.headers.file_name.as_deref(), Some("b.txt"));
    assert_eq!(second.bytes().await.expect("body bytes"), Bytes::from_static(b"two"));

    assert!(multipart
        .next_part()
        .await
        .expect("stream should finish")
        .is_none());
}

#[tokio::test]
async fn yields_first_part_before_input_completes() {
    let first_chunk = concat!(
        "--B\r\n",
        "Content-Disposition: form-data; name=\"first\"\r\n",
        "\r\n",
        "one\r\n",
        "--B\r\n",
        "Content-Disposition: form-data; name=\"second\"\r\n",
        "\r\n"
    );
    let second_chunk = concat!("two\r\n", "--B--\r\n");

    let (tx, rx) = mpsc::unbounded::<Result<Bytes, MulterError>>();
    let mut multipart = Multipart::new("B", rx).expect("boundary should be valid");

    tx.unbounded_send(Ok(Bytes::from_static(first_chunk.as_bytes())))
        .expect("send first chunk");

    let mut first = multipart
        .next_part()
        .await
        .expect("first part should parse")
        .expect("first item should exist");
    assert_eq!(first.headers.field_name, "first");
    assert_eq!(first.bytes().await.expect("body bytes"), Bytes::from_static(b"one"));

    tx.unbounded_send(Ok(Bytes::from_static(second_chunk.as_bytes())))
        .expect("send second chunk");
    drop(tx);

    let mut second = multipart
        .next_part()
        .await
        .expect("second part should parse")
        .expect("second item should exist");
    assert_eq!(second.headers.field_name, "second");
    assert_eq!(second.bytes().await.expect("body bytes"), Bytes::from_static(b"two"));
    assert!(multipart
        .next_part()
        .await
        .expect("stream should finish")
        .is_none());
}

#[tokio::test]
async fn reports_malformed_boundary_as_parse_error() {
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "hello\r\n",
        "--WRONG--\r\n"
    );
    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");

    let mut item = multipart
        .next_part()
        .await
        .expect("headers should parse")
        .expect("item expected");
    let item = item.bytes().await.expect_err("body should fail");
    assert!(matches!(
        item,
        MulterError::Parse(ParseError::Message { .. })
    ));
}

#[tokio::test]
async fn reports_incomplete_terminal_boundary() {
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "hello"
    );
    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");

    let mut item = multipart
        .next_part()
        .await
        .expect("headers should parse")
        .expect("item expected");
    let item = item.bytes().await.expect_err("body should fail");
    assert!(matches!(item, MulterError::IncompleteStream));
}

#[tokio::test]
async fn reports_invalid_headers_as_parse_error() {
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition form-data; name=\"field\"\r\n",
        "\r\n",
        "hello\r\n",
        "--BOUND--\r\n"
    );
    let input = stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]);
    let mut multipart = Multipart::new("BOUND", input).expect("boundary should be valid");

    let item = multipart.next_part().await.expect_err("item expected");
    assert!(matches!(
        item,
        MulterError::Parse(ParseError::Message { .. })
    ));
}

fn split_bytes(input: &[u8], chunk_sizes: &[usize]) -> Vec<Bytes> {
    let mut chunks = Vec::new();
    let mut index = 0usize;

    for &size in chunk_sizes {
        if index >= input.len() {
            break;
        }
        let end = (index + size).min(input.len());
        chunks.push(Bytes::copy_from_slice(&input[index..end]));
        index = end;
    }

    if index < input.len() {
        chunks.push(Bytes::copy_from_slice(&input[index..]));
    }

    chunks
}

#[tokio::test]
async fn streams_large_body_before_terminal_boundary_arrives() {
    let (tx, rx) = mpsc::unbounded::<Result<Bytes, MulterError>>();
    let mut multipart = Multipart::new("BOUND", rx).expect("boundary should be valid");

    tx.unbounded_send(Ok(Bytes::from_static(
        b"--BOUND\r\nContent-Disposition: form-data; name=\"upload\"; filename=\"big.bin\"\r\n\r\n",
    )))
    .expect("send prelude");
    tx.unbounded_send(Ok(Bytes::from(vec![b'x'; 128 * 1024])))
        .expect("send first payload chunk");

    let mut part = multipart
        .next_part()
        .await
        .expect("headers should parse")
        .expect("part should exist");

    let mut stream = part.stream();
    let first = stream
        .next()
        .await
        .expect("chunk should exist")
        .expect("chunk should parse");
    assert!(first.len() >= 64 * 1024);

    tx.unbounded_send(Ok(Bytes::from(vec![b'y'; 128 * 1024])))
        .expect("send second payload chunk");
    tx.unbounded_send(Ok(Bytes::from_static(b"\r\n--BOUND--\r\n")))
        .expect("send trailer");
    drop(tx);

    let mut total = first.len();
    while let Some(chunk) = stream.next().await {
        total += chunk.expect("chunk should parse").len();
    }

    assert_eq!(total, 256 * 1024);
}

