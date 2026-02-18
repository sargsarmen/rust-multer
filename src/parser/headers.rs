use http::{HeaderMap, header};

use crate::error::ParseError;

const DEFAULT_PART_CONTENT_TYPE: &str = "application/octet-stream";

/// Parsed `Content-Disposition` metadata for a multipart part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDisposition {
    /// Disposition type, typically `form-data`.
    pub disposition: String,
    /// Parsed field name (`name` parameter).
    pub name: Option<String>,
    /// Parsed file name (`filename`/`filename*` parameter).
    pub filename: Option<String>,
}

/// Parsed header model for a multipart part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPartHeaders {
    /// Parsed content disposition metadata.
    pub content_disposition: ContentDisposition,
    /// Logical field name for this part.
    pub field_name: String,
    /// Optional file name, if this part represents a file field.
    pub file_name: Option<String>,
    /// Parsed part-level content type.
    pub content_type: mime::Mime,
}

/// Parses a multipart part `Content-Disposition` value.
pub fn parse_content_disposition(value: &str) -> Result<ContentDisposition, ParseError> {
    let mut segments = split_semicolon_aware(value).into_iter();
    let disposition = segments
        .next()
        .map(|segment| segment.trim().to_ascii_lowercase())
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| ParseError::new("invalid Content-Disposition header"))?;

    let mut name: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut filename_star: Option<String> = None;

    for segment in segments {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((raw_key, raw_value)) = trimmed.split_once('=') else {
            return Err(ParseError::new(
                "invalid Content-Disposition parameter format",
            ));
        };

        let key = raw_key.trim().to_ascii_lowercase();
        let decoded = parse_parameter_value(raw_value.trim())?;

        match key.as_str() {
            "name" => name = Some(decoded),
            "filename" => filename = Some(parse_filename_value(&decoded)?),
            "filename*" => filename_star = Some(parse_rfc5987_value(&decoded)?),
            _ => {}
        }
    }

    if disposition == "form-data" && matches!(name.as_deref(), None | Some("")) {
        return Err(ParseError::new(
            "form-data Content-Disposition must include non-empty `name`",
        ));
    }

    Ok(ContentDisposition {
        disposition,
        name,
        filename: filename_star.or(filename),
    })
}

/// Parses part-level `Content-Type`, defaulting to `application/octet-stream`.
pub fn parse_part_content_type(value: Option<&str>) -> Result<mime::Mime, ParseError> {
    let raw = value.unwrap_or(DEFAULT_PART_CONTENT_TYPE).trim();
    raw.parse::<mime::Mime>()
        .map_err(|_| ParseError::new("invalid part Content-Type header"))
}

/// Parses multipart part headers needed by higher-level parser stages.
pub fn parse_part_headers(headers: &HeaderMap) -> Result<ParsedPartHeaders, ParseError> {
    let disposition_raw = headers
        .get(header::CONTENT_DISPOSITION)
        .ok_or_else(|| ParseError::new("missing Content-Disposition header"))?;

    let disposition_raw = disposition_raw
        .to_str()
        .map_err(|_| ParseError::new("Content-Disposition header must be ASCII"))?;
    let content_disposition = parse_content_disposition(disposition_raw)?;

    let field_name = content_disposition
        .name
        .clone()
        .ok_or_else(|| ParseError::new("missing part field name"))?;

    let content_type_raw = headers
        .get(header::CONTENT_TYPE)
        .map(|value| {
            value
                .to_str()
                .map_err(|_| ParseError::new("Content-Type header must be ASCII"))
        })
        .transpose()?;

    let content_type = parse_part_content_type(content_type_raw)?;

    Ok(ParsedPartHeaders {
        file_name: content_disposition.filename.clone(),
        content_disposition,
        field_name,
        content_type,
    })
}

fn parse_parameter_value(raw: &str) -> Result<String, ParseError> {
    if let Some(stripped) = raw.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        return unescape_quoted_string(stripped);
    }

    if raw.contains('"') {
        return Err(ParseError::new("invalid quoted parameter value"));
    }

    Ok(raw.trim().to_owned())
}

fn unescape_quoted_string(value: &str) -> Result<String, ParseError> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let escaped = chars
                .next()
                .ok_or_else(|| ParseError::new("dangling escape in quoted parameter"))?;
            out.push(escaped);
            continue;
        }
        out.push(ch);
    }

    Ok(out)
}

fn parse_rfc5987_value(value: &str) -> Result<String, ParseError> {
    let Some((charset, encoded)) = split_rfc5987(value) else {
        return Err(ParseError::new("invalid filename* parameter encoding"));
    };

    if !charset.eq_ignore_ascii_case("utf-8") {
        return Err(ParseError::new("only UTF-8 filename* charset is supported"));
    }

    percent_decode_utf8(
        encoded,
        "invalid percent-encoding in filename*",
        "filename* is not valid UTF-8",
    )
}

fn split_rfc5987(value: &str) -> Option<(&str, &str)> {
    let (charset, rest) = value.split_once('\'')?;
    let (_, encoded) = rest.split_once('\'')?;
    Some((charset, encoded))
}

fn parse_filename_value(value: &str) -> Result<String, ParseError> {
    if !value.as_bytes().contains(&b'%') {
        return Ok(value.to_owned());
    }

    percent_decode_utf8(
        value,
        "invalid percent-encoding in filename",
        "filename is not valid UTF-8",
    )
}

fn percent_decode_utf8(
    value: &str,
    invalid_encoding_message: &'static str,
    invalid_utf8_message: &'static str,
) -> Result<String, ParseError> {
    let mut bytes = Vec::with_capacity(value.len());
    let raw = value.as_bytes();
    let mut index = 0;

    while index < raw.len() {
        if raw[index] == b'%' {
            if index + 2 >= raw.len() {
                return Err(ParseError::new(invalid_encoding_message));
            }
            let hi = hex_value(raw[index + 1], invalid_encoding_message)?;
            let lo = hex_value(raw[index + 2], invalid_encoding_message)?;
            bytes.push((hi << 4) | lo);
            index += 3;
            continue;
        }

        bytes.push(raw[index]);
        index += 1;
    }

    String::from_utf8(bytes).map_err(|_| ParseError::new(invalid_utf8_message))
}

fn hex_value(byte: u8, invalid_encoding_message: &'static str) -> Result<u8, ParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseError::new(invalid_encoding_message)),
    }
}

fn split_semicolon_aware(value: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => {
                current.push(ch);
                escaped = true;
            }
            '"' => {
                current.push(ch);
                in_quotes = !in_quotes;
            }
            ';' if !in_quotes => {
                segments.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }

    segments.push(current);
    segments
}
