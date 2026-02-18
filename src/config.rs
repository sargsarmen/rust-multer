use std::collections::HashSet;

use crate::{error::ConfigError, limits::Limits};

/// Allowed file field declaration for `fields(...)` selector mode.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedField {
    /// Logical field name.
    pub name: String,
    /// Maximum file count accepted for this field.
    pub max_count: Option<usize>,
    /// Allowed MIME patterns for this field (for example: `image/*`).
    pub allowed_mime_types: Vec<String>,
}

impl SelectedField {
    /// Creates a selected field with no explicit per-field max count.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_count: None,
            allowed_mime_types: Vec::new(),
        }
    }

    /// Sets the maximum file count accepted for this field.
    pub fn with_max_count(mut self, max_count: usize) -> Self {
        self.max_count = Some(max_count);
        self
    }

    /// Alias for [`SelectedField::with_max_count`].
    pub fn max_count(self, max_count: usize) -> Self {
        self.with_max_count(max_count)
    }

    /// Sets MIME patterns accepted for this field.
    pub fn with_allowed_mime_types<I, M>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<String>,
    {
        self.allowed_mime_types = patterns.into_iter().map(Into::into).collect();
        self
    }

    /// Alias for [`SelectedField::with_allowed_mime_types`].
    pub fn allowed_mime_types<I, M>(self, patterns: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<String>,
    {
        self.with_allowed_mime_types(patterns)
    }

    /// Validates a single selected field configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            return Err(ConfigError::EmptyFieldName);
        }

        if matches!(self.max_count, Some(0)) {
            return Err(ConfigError::InvalidFieldMaxCount {
                name: self.name.clone(),
            });
        }

        for pattern in &self.allowed_mime_types {
            if !is_valid_mime_pattern(pattern) {
                return Err(ConfigError::InvalidMimePattern {
                    pattern: pattern.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Strategy for matching incoming file fields.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Validates selector-specific constraints.
    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            Self::Single { name } => {
                validate_field_name(name)?;
            }
            Self::Array { name, max_count } => {
                validate_field_name(name)?;
                if matches!(max_count, Some(0)) {
                    return Err(ConfigError::InvalidArrayMaxCount { name: name.clone() });
                }
            }
            Self::Fields(fields) => {
                if fields.is_empty() {
                    return Err(ConfigError::EmptyFieldsSelector);
                }

                let mut seen = HashSet::with_capacity(fields.len());
                for field in fields {
                    field.validate()?;
                    if !seen.insert(field.name.clone()) {
                        return Err(ConfigError::DuplicateFieldName {
                            name: field.name.clone(),
                        });
                    }
                }
            }
            Self::None | Self::Any => {}
        }

        Ok(())
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::Any
    }
}

/// Policy for handling fields not described by the active selector.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownFieldPolicy {
    /// Reject unknown fields with an error.
    Reject,
    /// Ignore unknown fields.
    #[default]
    Ignore,
}

/// Top-level multipart configuration model.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Validates selector and limit configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.selector.validate()?;
        validate_limits(&self.limits)?;
        Ok(())
    }
}

fn validate_field_name(name: &str) -> Result<(), ConfigError> {
    if name.trim().is_empty() {
        return Err(ConfigError::EmptyFieldName);
    }

    Ok(())
}

fn validate_limits(limits: &Limits) -> Result<(), ConfigError> {
    validate_positive_u64("max_file_size", limits.max_file_size)?;
    validate_positive_usize("max_files", limits.max_files)?;
    validate_positive_u64("max_field_size", limits.max_field_size)?;
    validate_positive_usize("max_fields", limits.max_fields)?;
    validate_positive_u64("max_body_size", limits.max_body_size)?;

    if let Some(max_body_size) = limits.max_body_size {
        if let Some(max_file_size) = limits.max_file_size {
            if max_file_size > max_body_size {
                return Err(ConfigError::LimitExceedsBodySize {
                    limit: "max_file_size",
                    value: max_file_size,
                    max_body_size,
                });
            }
        }

        if let Some(max_field_size) = limits.max_field_size {
            if max_field_size > max_body_size {
                return Err(ConfigError::LimitExceedsBodySize {
                    limit: "max_field_size",
                    value: max_field_size,
                    max_body_size,
                });
            }
        }
    }

    for pattern in &limits.allowed_mime_types {
        if !is_valid_mime_pattern(pattern) {
            return Err(ConfigError::InvalidMimePattern {
                pattern: pattern.clone(),
            });
        }
    }

    Ok(())
}

fn validate_positive_u64(limit: &'static str, value: Option<u64>) -> Result<(), ConfigError> {
    if matches!(value, Some(0)) {
        return Err(ConfigError::InvalidLimitValue { limit });
    }

    Ok(())
}

fn validate_positive_usize(limit: &'static str, value: Option<usize>) -> Result<(), ConfigError> {
    if matches!(value, Some(0)) {
        return Err(ConfigError::InvalidLimitValue { limit });
    }

    Ok(())
}

fn is_valid_mime_pattern(pattern: &str) -> bool {
    let Some((kind, subtype)) = pattern.split_once('/') else {
        return false;
    };

    if kind.is_empty() || subtype.is_empty() {
        return false;
    }

    if subtype == "*" {
        return kind.chars().all(is_valid_mime_token_char);
    }

    pattern.parse::<mime::Mime>().is_ok()
}

fn is_valid_mime_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '!' | '#' | '$' | '&' | '-' | '^' | '_' | '.' | '+')
}
