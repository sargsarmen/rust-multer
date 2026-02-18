/// Request and field limits enforced during multipart parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// Maximum accepted file size in bytes for a single file part.
    pub max_file_size: Option<u64>,
    /// Maximum total number of file parts in a request.
    pub max_files: Option<usize>,
    /// Maximum accepted size in bytes for a text field.
    pub max_field_size: Option<u64>,
    /// Maximum number of text fields in a request.
    pub max_fields: Option<usize>,
    /// Maximum request body size in bytes.
    pub max_body_size: Option<u64>,
    /// Allowed MIME patterns (for example: `image/png`, `image/*`).
    pub allowed_mime_types: Vec<String>,
}

impl Limits {
    /// Creates a permissive limits configuration.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_size: None,
            max_files: None,
            max_field_size: None,
            max_fields: None,
            max_body_size: None,
            allowed_mime_types: Vec::new(),
        }
    }
}
