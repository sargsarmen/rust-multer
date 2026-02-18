use crate::limits::Limits;

/// Allowed file field declaration for `fields(...)` selector mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedField {
    /// Logical field name.
    pub name: String,
    /// Maximum file count accepted for this field.
    pub max_count: Option<usize>,
}

impl SelectedField {
    /// Creates a selected field with no explicit per-field max count.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_count: None,
        }
    }

    /// Sets the maximum file count accepted for this field.
    pub fn with_max_count(mut self, max_count: usize) -> Self {
        self.max_count = Some(max_count);
        self
    }
}

/// Strategy for matching incoming file fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// Accept a single file for one named field.
    Single {
        /// Target field name.
        name: String,
    },
    /// Accept multiple files for one named field.
    Array {
        /// Target field name.
        name: String,
        /// Maximum number of files accepted for this field.
        max_count: Option<usize>,
    },
    /// Accept files for a set of named fields.
    Fields(Vec<SelectedField>),
    /// Reject all file parts.
    None,
    /// Accept files for any field name.
    Any,
}

impl Selector {
    /// Creates a selector that allows one file for the given field name.
    pub fn single(name: impl Into<String>) -> Self {
        Self::Single { name: name.into() }
    }

    /// Creates a selector that allows up to `max_count` files for a field name.
    pub fn array(name: impl Into<String>, max_count: usize) -> Self {
        Self::Array {
            name: name.into(),
            max_count: Some(max_count),
        }
    }

    /// Creates a selector that allows files for multiple named fields.
    pub fn fields(fields: impl IntoIterator<Item = SelectedField>) -> Self {
        Self::Fields(fields.into_iter().collect())
    }

    /// Creates a selector that rejects all file uploads.
    pub fn none() -> Self {
        Self::None
    }

    /// Creates a selector that allows file uploads for any field name.
    pub fn any() -> Self {
        Self::Any
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::Any
    }
}

/// Policy for handling fields not described by the active selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownFieldPolicy {
    /// Reject unknown fields with an error.
    Reject,
    /// Ignore unknown fields.
    #[default]
    Ignore,
}

/// Top-level multipart configuration model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MulterConfig {
    /// Selector strategy for file fields.
    pub selector: Selector,
    /// Behavior when an incoming field does not match the selector.
    pub unknown_field_policy: UnknownFieldPolicy,
    /// Global request limits.
    pub limits: Limits,
}

impl MulterConfig {
    /// Creates a default configuration.
    pub fn new() -> Self {
        Self::default()
    }
}
