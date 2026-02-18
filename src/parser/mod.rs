/// Multipart boundary parsing helpers.
pub mod boundary;
/// Multipart part header parsing helpers.
pub mod headers;
/// Streaming multipart parser state machine.
pub mod stream;

pub use boundary::extract_multipart_boundary;
pub use headers::{
    ContentDisposition, ParsedPartHeaders, parse_content_disposition, parse_part_content_type,
    parse_part_headers,
};
pub use stream::{MultipartStream, ParsedPart};

/// Low-level multipart parser entry type.
#[derive(Debug, Clone, Default)]
pub struct Parser;
