/// Request and field limits enforced during multipart parsing.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

    /// Returns `true` when `mime` is allowed by the configured allowlist.
    ///
    /// When no allowlist is configured, all MIME types are accepted.
    pub fn is_mime_allowed(&self, mime: &mime::Mime) -> bool {
        if self.allowed_mime_types.is_empty() {
            return true;
        }

        let allowed = self
            .allowed_mime_types
            .iter()
            .any(|pattern| mime_matches_pattern(mime, pattern));

        #[cfg(feature = "tracing")]
        if !allowed {
            tracing::debug!(
                mime = mime.essence_str(),
                allowed_patterns = ?self.allowed_mime_types,
                "limits: MIME rejected by global allowlist"
            );
        }

        allowed
    }
}

fn mime_matches_pattern(mime: &mime::Mime, pattern: &str) -> bool {
    if let Some((kind, subtype)) = pattern.split_once('/') {
        if subtype == "*" {
            return mime.type_().as_str().eq_ignore_ascii_case(kind);
        }
    }

    mime.essence_str().eq_ignore_ascii_case(pattern)
}
