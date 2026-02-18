use crate::error::ParseError;

const MULTIPART_FORM_DATA: &str = "multipart/form-data";
const MAX_BOUNDARY_LEN: usize = 70;

/// Extracts and validates the `boundary` parameter from a `Content-Type` value.
pub fn extract_multipart_boundary(content_type: &str) -> Result<String, ParseError> {
    let mime = content_type
        .parse::<mime::Mime>()
        .map_err(|_| ParseError::new("invalid Content-Type header"))?;

    if mime.essence_str() != MULTIPART_FORM_DATA {
        return Err(ParseError::new("Content-Type must be multipart/form-data"));
    }

    let boundary = mime
        .get_param("boundary")
        .map(|value| value.as_str())
        .ok_or_else(|| ParseError::new("missing multipart boundary parameter"))?;

    let boundary = decode_boundary_percent_encoding(boundary)?;
    validate_boundary(&boundary)?;
    Ok(boundary)
}

fn validate_boundary(boundary: &str) -> Result<(), ParseError> {
    if boundary.is_empty() {
        return Err(ParseError::new("multipart boundary cannot be empty"));
    }

    if boundary.len() > MAX_BOUNDARY_LEN {
        return Err(ParseError::new("multipart boundary cannot exceed 70 characters"));
    }

    if boundary.ends_with(' ') {
        return Err(ParseError::new(
            "multipart boundary cannot end with whitespace",
        ));
    }

    if !boundary.chars().all(is_boundary_char) {
        return Err(ParseError::new(
            "multipart boundary contains invalid characters",
        ));
    }

    Ok(())
}

fn decode_boundary_percent_encoding(boundary: &str) -> Result<String, ParseError> {
    if !boundary.as_bytes().contains(&b'%') {
        return Ok(boundary.to_owned());
    }

    let mut bytes = Vec::with_capacity(boundary.len());
    let raw = boundary.as_bytes();
    let mut index = 0usize;

    while index < raw.len() {
        if raw[index] == b'%' {
            if index + 2 >= raw.len() {
                return Err(ParseError::new("invalid percent-encoding in multipart boundary"));
            }

            let hi = hex_value(raw[index + 1])?;
            let lo = hex_value(raw[index + 2])?;
            bytes.push((hi << 4) | lo);
            index += 3;
            continue;
        }

        bytes.push(raw[index]);
        index += 1;
    }

    String::from_utf8(bytes)
        .map_err(|_| ParseError::new("multipart boundary percent-encoding is not valid UTF-8"))
}

fn hex_value(byte: u8) -> Result<u8, ParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseError::new("invalid percent-encoding in multipart boundary")),
    }
}

fn is_boundary_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '\'' | '(' | ')' | '+' | '_' | ',' | '-' | '.' | '/' | ':' | '=' | '?' | ' ')
}
