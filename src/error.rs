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

/// Runtime error type used by `rust-multer`.
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
    /// Multipart stream ended before a complete terminal boundary.
    #[error("multipart stream ended unexpectedly")]
    IncompleteStream,
}
