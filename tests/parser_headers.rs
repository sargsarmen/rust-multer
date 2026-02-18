#![allow(missing_docs)]

use http::{HeaderMap, HeaderValue, header};
use multigear::parser::headers::{
    parse_content_disposition, parse_part_content_type, parse_part_headers,
};

#[test]
fn parses_content_disposition_name_and_filename() {
    let parsed = parse_content_disposition("form-data; name=\"avatar\"; filename=\"face.png\"")
        .expect("header should parse");

    assert_eq!(parsed.disposition, "form-data");
    assert_eq!(parsed.name.as_deref(), Some("avatar"));
    assert_eq!(parsed.filename.as_deref(), Some("face.png"));
}

#[test]
fn parses_escaped_quoted_values() {
    let parsed =
        parse_content_disposition("form-data; name=\"fi\\\"eld\"; filename=\"te\\\\st.txt\"")
            .expect("header should parse");

    assert_eq!(parsed.name.as_deref(), Some("fi\"eld"));
    assert_eq!(parsed.filename.as_deref(), Some("te\\st.txt"));
}

#[test]
fn parses_percent_encoded_filename_parameter() {
    let parsed = parse_content_disposition("form-data; name=\"file\"; filename=\"hello%20world.txt\"")
        .expect("header should parse");
    assert_eq!(parsed.filename.as_deref(), Some("hello world.txt"));
}

#[test]
fn filename_star_takes_precedence_over_filename() {
    let parsed = parse_content_disposition(
        "form-data; name=\"upload\"; filename=\"fallback.txt\"; filename*=UTF-8''real%20name.txt",
    )
    .expect("header should parse");

    assert_eq!(parsed.filename.as_deref(), Some("real name.txt"));
}

#[test]
fn defaults_part_content_type_to_octet_stream() {
    let mime = parse_part_content_type(None).expect("default MIME should parse");
    assert_eq!(mime.essence_str(), "application/octet-stream");
}

#[test]
fn parses_explicit_part_content_type() {
    let mime = parse_part_content_type(Some("text/plain; charset=utf-8"))
        .expect("explicit MIME should parse");
    assert_eq!(mime.essence_str(), "text/plain");
    assert_eq!(
        mime.get_param("charset").map(|v| v.as_str()),
        Some("utf-8")
    );
}

#[test]
fn parse_part_headers_extracts_core_values() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("form-data; name=\"avatar\"; filename=\"face.png\""),
    );
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/png"),
    );

    let parsed = parse_part_headers(&headers).expect("part headers should parse");
    assert_eq!(parsed.field_name, "avatar");
    assert_eq!(parsed.file_name.as_deref(), Some("face.png"));
    assert_eq!(parsed.content_type.essence_str(), "image/png");
}

#[test]
fn rejects_missing_content_disposition_header() {
    let headers = HeaderMap::new();
    let err = parse_part_headers(&headers).expect_err("must fail");
    assert_err_contains(&err.to_string(), "missing Content-Disposition");
}

#[test]
fn rejects_malformed_content_disposition() {
    let err = parse_content_disposition("form-data; name").expect_err("must fail");
    assert_err_contains(&err.to_string(), "parameter format");
}

#[test]
fn rejects_form_data_without_non_empty_name() {
    let err = parse_content_disposition("form-data; name=\"\"").expect_err("must fail");
    assert_err_contains(&err.to_string(), "non-empty `name`");
}

#[test]
fn rejects_invalid_part_content_type() {
    let err = parse_part_content_type(Some("not-a/type?")).expect_err("must fail");
    assert_err_contains(&err.to_string(), "invalid part Content-Type");
}

#[test]
fn rejects_malformed_percent_encoding_in_filename_parameter() {
    let err = parse_content_disposition("form-data; name=\"file\"; filename=\"bad%2\"")
        .expect_err("must fail");
    assert_err_contains(&err.to_string(), "percent-encoding");
}

fn assert_err_contains(actual: &str, expected_fragment: &str) {
    assert!(
        actual.contains(expected_fragment),
        "expected `{actual}` to contain `{expected_fragment}`"
    );
}

