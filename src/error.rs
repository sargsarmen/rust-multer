use thiserror::Error;

/// Configuration-time validation errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// A selector field name was empty.
    #[error("selector field name cannot be empty")]
    EmptyFieldName,
    /// A field has an invalid `max_count` of zero.
    #[error("field `{name}` has invalid max_count of 0")]
    InvalidFieldMaxCount {
        /// Name of the field with an invalid count.
        name: String,
    },
    /// A field has an invalid `max_size` of zero.
    #[error("field `{name}` has invalid max_size of 0")]
    InvalidFieldMaxSize {
        /// Name of the field with an invalid size.
        name: String,
    },
    /// An `array(...)` selector has an invalid `max_count` of zero.
    #[error("array selector for field `{name}` has invalid max_count of 0")]
    InvalidArrayMaxCount {
        /// Name of the field with an invalid count.
        name: String,
    },
    /// The `fields(...)` selector was configured with no fields.
    #[error("fields selector must contain at least one field")]
    EmptyFieldsSelector,
    /// The `fields(...)` selector contains duplicate names.
    #[error("duplicate field `{name}` in fields selector")]
    DuplicateFieldName {
        /// Duplicated field name.
        name: String,
    },
    /// A configured numeric limit must be strictly greater than zero.
    #[error("limit `{limit}` must be greater than 0")]
    InvalidLimitValue {
        /// Name of the limit.
        limit: &'static str,
    },
    /// A per-part limit exceeded the configured body limit.
    #[error("limit `{limit}` ({value}) cannot exceed `max_body_size` ({max_body_size})")]
    LimitExceedsBodySize {
        /// Name of the limit that exceeded `max_body_size`.
        limit: &'static str,
        /// Configured value of `limit`.
        value: u64,
        /// Configured `max_body_size`.
        max_body_size: u64,
    },
    /// An allowed MIME pattern is malformed.
    #[error("invalid MIME pattern `{pattern}`")]
    InvalidMimePattern {
        /// The invalid pattern value.
        pattern: String,
    },
}

/// Parser-level multipart failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// Generic parser failure with message context.
    #[error("{message}")]
    Message {
        /// Parser failure message.
        message: String,
    },
}

impl ParseError {
    /// Creates a parser error from a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

/// Storage backend failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StorageError {
    /// Generic storage failure with message context.
    #[error("{message}")]
    Message {
        /// Storage failure message.
        message: String,
    },
}

impl StorageError {
    /// Creates a storage error from a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
        }
    }
}

/// Runtime error type used by `multigear`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MulterError {
    /// Configuration error surfaced at runtime.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Multipart parser failure.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// Storage backend failure.
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// Incoming field does not match active selector configuration.
    #[error("unexpected field `{field}`")]
    UnexpectedField {
        /// Field name encountered in the stream.
        field: String,
    },
    /// File count for a field exceeded the active selector limit.
    #[error("field `{field}` exceeded max count of {max_count}")]
    FieldCountLimitExceeded {
        /// Field name that exceeded its file-count limit.
        field: String,
        /// Maximum allowed file count for this field.
        max_count: usize,
    },
    /// A file part exceeded the configured size limit.
    #[error("file field `{field}` exceeded max file size of {max_file_size} bytes")]
    FileSizeLimitExceeded {
        /// Field name that exceeded the file-size limit.
        field: String,
        /// Maximum allowed file size in bytes.
        max_file_size: u64,
    },
    /// A text part exceeded the configured size limit.
    #[error("text field `{field}` exceeded max field size of {max_field_size} bytes")]
    FieldSizeLimitExceeded {
        /// Field name that exceeded the text-size limit.
        field: String,
        /// Maximum allowed text field size in bytes.
        max_field_size: u64,
    },
    /// The number of accepted file parts exceeded the configured limit.
    #[error("multipart request exceeded max files limit of {max_files}")]
    FilesLimitExceeded {
        /// Maximum allowed number of file parts.
        max_files: usize,
    },
    /// The number of accepted text parts exceeded the configured limit.
    #[error("multipart request exceeded max fields limit of {max_fields}")]
    FieldsLimitExceeded {
        /// Maximum allowed number of text parts.
        max_fields: usize,
    },
    /// The request body exceeded the configured body-size limit.
    #[error("multipart request exceeded max body size of {max_body_size} bytes")]
    BodySizeLimitExceeded {
        /// Maximum allowed request body size in bytes.
        max_body_size: u64,
    },
    /// A file MIME type is not permitted by the configured allowlist.
    #[error("file field `{field}` has disallowed MIME type `{mime}`")]
    MimeTypeNotAllowed {
        /// File field name.
        field: String,
        /// MIME type encountered for the file part.
        mime: String,
    },
    /// Multipart stream ended before a complete terminal boundary.
    #[error("multipart stream ended unexpectedly")]
    IncompleteStream,
}

