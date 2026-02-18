#![allow(missing_docs)]

use rust_multer::parser::boundary::extract_multipart_boundary;

#[test]
fn extracts_boundary_from_content_type() {
    let boundary = extract_multipart_boundary("multipart/form-data; boundary=abc123")
        .expect("boundary should parse");
    assert_eq!(boundary, "abc123");
}

#[test]
fn extracts_quoted_boundary() {
    let boundary = extract_multipart_boundary("multipart/form-data; boundary=\"my-boundary\"")
        .expect("quoted boundary should parse");
    assert_eq!(boundary, "my-boundary");
}

#[test]
fn rejects_non_multipart_content_type() {
    let err = extract_multipart_boundary("application/json").expect_err("must fail");
    assert_err_contains(&err.to_string(), "multipart/form-data");
}

#[test]
fn rejects_missing_boundary_parameter() {
    let err = extract_multipart_boundary("multipart/form-data").expect_err("must fail");
    assert_err_contains(&err.to_string(), "missing multipart boundary");
}

#[test]
fn rejects_invalid_boundary_characters() {
    let err = extract_multipart_boundary("multipart/form-data; boundary=abc@123")
        .expect_err("must fail");
    assert_err_contains(&err.to_string(), "invalid");
}

#[test]
fn rejects_boundary_that_is_too_long() {
    let long_boundary = "a".repeat(71);
    let header = format!("multipart/form-data; boundary={long_boundary}");
    let err = extract_multipart_boundary(&header).expect_err("must fail");
    assert_err_contains(&err.to_string(), "cannot exceed 70");
}

#[test]
fn decodes_percent_encoded_boundary() {
    let boundary = extract_multipart_boundary("multipart/form-data; boundary=abc%2D123")
        .expect("boundary should parse");
    assert_eq!(boundary, "abc-123");
}

#[test]
fn rejects_malformed_percent_encoding_in_boundary() {
    let err = extract_multipart_boundary("multipart/form-data; boundary=abc%2")
        .expect_err("must fail");
    assert_err_contains(&err.to_string(), "percent-encoding");
}

fn assert_err_contains(actual: &str, expected_fragment: &str) {
    assert!(
        actual.contains(expected_fragment),
        "expected `{actual}` to contain `{expected_fragment}`"
    );
}
