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

    let first = multipart
        .next()
        .await
        .expect("first item should exist")
        .expect("first part should parse");
    assert_eq!(first.headers.field_name, "alpha");
    assert!(first.headers.file_name.is_none());
    assert_eq!(first.body, Bytes::from_static(b"one"));

    let second = multipart
        .next()
        .await
        .expect("second item should exist")
        .expect("second part should parse");
    assert_eq!(second.headers.field_name, "beta");
    assert_eq!(second.headers.file_name.as_deref(), Some("b.txt"));
    assert_eq!(second.body, Bytes::from_static(b"two"));

    assert!(multipart.next().await.is_none());
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

    let first = multipart
        .next()
        .await
        .expect("first item should exist")
        .expect("first part should parse");
    assert_eq!(first.headers.field_name, "first");
    assert_eq!(first.body, Bytes::from_static(b"one"));

    tx.unbounded_send(Ok(Bytes::from_static(second_chunk.as_bytes())))
        .expect("send second chunk");
    drop(tx);

    let second = multipart
        .next()
        .await
        .expect("second item should exist")
        .expect("second part should parse");
    assert_eq!(second.headers.field_name, "second");
    assert_eq!(second.body, Bytes::from_static(b"two"));
    assert!(multipart.next().await.is_none());
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

    let item = multipart.next().await.expect("item expected");
    assert!(matches!(
        item,
        Err(MulterError::Parse(ParseError::Message { .. }))
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

    let item = multipart.next().await.expect("item expected");
    assert!(matches!(item, Err(MulterError::IncompleteStream)));
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

    let item = multipart.next().await.expect("item expected");
    assert!(matches!(
        item,
        Err(MulterError::Parse(ParseError::Message { .. }))
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
