#![allow(missing_docs)]

use bytes::Bytes;
use futures::stream;
use multigear::{
    MulterConfig, MulterError, Multipart, SelectedField, Selector, UnknownFieldPolicy,
};

#[tokio::test]
async fn single_selector_rejects_second_file_for_same_field() {
    let config = MulterConfig {
        selector: Selector::single("avatar"),
        unknown_field_policy: UnknownFieldPolicy::Reject,
        ..MulterConfig::default()
    };
    let body = multipart_body(&[
        ("avatar", Some("a.png"), "one"),
        ("avatar", Some("b.png"), "two"),
    ]);

    let mut multipart = Multipart::with_config("BOUND", bytes_stream(body), config)
        .expect("multipart should initialize");

    let first = multipart
        .next_part()
        .await
        .expect("first item expected")
        .expect("first item should pass selector");
    assert_eq!(first.field_name(), "avatar");

    let second = multipart.next_part().await.expect_err("second item expected");
    assert!(matches!(
        second,
        MulterError::FieldCountLimitExceeded {
            field,
            max_count: 1
        } if field == "avatar"
    ));
}

#[tokio::test]
async fn array_selector_rejects_unknown_file_field() {
    let config = MulterConfig {
        selector: Selector::array("photos", 2),
        unknown_field_policy: UnknownFieldPolicy::Reject,
        ..MulterConfig::default()
    };
    let body = multipart_body(&[("avatar", Some("a.png"), "one")]);
    let mut multipart = Multipart::with_config("BOUND", bytes_stream(body), config)
        .expect("multipart should initialize");

    let item = multipart.next_part().await.expect_err("item expected");
    assert!(matches!(
        item,
        MulterError::UnexpectedField { field } if field == "avatar"
    ));
}

#[tokio::test]
async fn fields_selector_enforces_per_field_max_counts() {
    let config = MulterConfig {
        selector: Selector::fields([
            SelectedField::new("docs").with_max_count(1),
            SelectedField::new("images").with_max_count(2),
        ]),
        unknown_field_policy: UnknownFieldPolicy::Reject,
        ..MulterConfig::default()
    };
    let body = multipart_body(&[
        ("docs", Some("a.txt"), "one"),
        ("images", Some("1.png"), "two"),
        ("images", Some("2.png"), "three"),
        ("images", Some("3.png"), "four"),
    ]);
    let mut multipart = Multipart::with_config("BOUND", bytes_stream(body), config)
        .expect("multipart should initialize");

    assert_eq!(
        multipart
            .next_part()
            .await
            .expect("item expected")
            .expect("item should pass selector")
            .field_name(),
        "docs"
    );
    assert_eq!(
        multipart
            .next_part()
            .await
            .expect("item expected")
            .expect("item should pass selector")
            .field_name(),
        "images"
    );
    assert_eq!(
        multipart
            .next_part()
            .await
            .expect("item expected")
            .expect("item should pass selector")
            .field_name(),
        "images"
    );

    let item = multipart.next_part().await.expect_err("item expected");
    assert!(matches!(
        item,
        MulterError::FieldCountLimitExceeded {
            field,
            max_count: 2
        } if field == "images"
    ));
}

#[tokio::test]
async fn none_selector_with_ignore_policy_skips_files_but_keeps_text_fields() {
    let config = MulterConfig {
        selector: Selector::none(),
        unknown_field_policy: UnknownFieldPolicy::Ignore,
        ..MulterConfig::default()
    };
    let body = multipart_body(&[
        ("avatar", Some("a.png"), "file-one"),
        ("note", None, "hello"),
        ("backup", Some("b.png"), "file-two"),
    ]);
    let mut multipart = Multipart::with_config("BOUND", bytes_stream(body), config)
        .expect("multipart should initialize");

    let mut names = Vec::new();
    loop {
        let next = multipart.next_part().await.expect("next part should parse");
        let Some(part) = next else {
            break;
        };
        names.push(part.field_name().to_owned());
    }

    assert_eq!(names, vec!["note"]);
}

#[tokio::test]
async fn any_selector_accepts_all_file_fields() {
    let config = MulterConfig {
        selector: Selector::any(),
        unknown_field_policy: UnknownFieldPolicy::Reject,
        ..MulterConfig::default()
    };
    let body = multipart_body(&[
        ("a", Some("a.bin"), "one"),
        ("b", Some("b.bin"), "two"),
    ]);
    let mut multipart = Multipart::with_config("BOUND", bytes_stream(body), config)
        .expect("multipart should initialize");

    let mut names = Vec::new();
    loop {
        let next = multipart.next_part().await.expect("all parts should be accepted");
        let Some(part) = next else {
            break;
        };
        names.push(part.field_name().to_owned());
    }

    assert_eq!(names, vec!["a", "b"]);
}

fn multipart_body(parts: &[(&str, Option<&str>, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (field, file_name, body) in parts {
        out.extend_from_slice(b"--BOUND\r\n");
        match file_name {
            Some(file_name) => {
                let disposition = format!(
                    "Content-Disposition: form-data; name=\"{field}\"; filename=\"{file_name}\"\r\n"
                );
                out.extend_from_slice(disposition.as_bytes());
                out.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
            }
            None => {
                let disposition = format!("Content-Disposition: form-data; name=\"{field}\"\r\n\r\n");
                out.extend_from_slice(disposition.as_bytes());
            }
        }
        out.extend_from_slice(body.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"--BOUND--\r\n");
    out
}

fn bytes_stream(body: Vec<u8>) -> impl futures::Stream<Item = Result<Bytes, MulterError>> {
    stream::iter([Ok(Bytes::from(body))])
}


