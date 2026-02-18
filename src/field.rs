/// Multipart field model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    /// File upload field metadata.
    File(FileField),
    /// Text field metadata.
    Text(TextField),
}

impl Field {
    /// Creates a file field model for the provided name.
    pub fn file(name: impl Into<String>) -> Self {
        Self::File(FileField::new(name))
    }

    /// Creates a text field model for the provided name.
    pub fn text(name: impl Into<String>) -> Self {
        Self::Text(TextField::new(name))
    }

    /// Returns the logical field name.
    pub fn name(&self) -> &str {
        match self {
            Self::File(field) => &field.name,
            Self::Text(field) => &field.name,
        }
    }

    /// Returns the field kind.
    pub fn kind(&self) -> FieldKind {
        match self {
            Self::File(_) => FieldKind::File,
            Self::Text(_) => FieldKind::Text,
        }
    }
}

/// Discriminates between file and text fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Binary file payload.
    File,
    /// Plain text payload.
    Text,
}

/// File field metadata and constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileField {
    /// Logical field name.
    pub name: String,
    /// Maximum number of file parts accepted for this field.
    pub max_count: Option<usize>,
}

impl FileField {
    /// Creates a file field with no explicit per-field count limit.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_count: None,
        }
    }

    /// Sets the maximum number of file parts for this field.
    pub fn with_max_count(mut self, max_count: usize) -> Self {
        self.max_count = Some(max_count);
        self
    }
}

/// Text field metadata and constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField {
    /// Logical field name.
    pub name: String,
    /// Maximum accepted text length in bytes.
    pub max_length: Option<usize>,
}

impl TextField {
    /// Creates a text field with no explicit per-field size limit.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_length: None,
        }
    }

    /// Sets the maximum text length in bytes for this field.
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }
}
