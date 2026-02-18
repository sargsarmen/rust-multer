#![allow(missing_docs)]

use rust_multer::{
    ConfigError, Limits, MulterBuilder, MulterConfig, SelectedField, Selector, UnknownFieldPolicy,
};

#[test]
fn rejects_empty_single_selector_name() {
    let config = MulterConfig {
        selector: Selector::single("   "),
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::EmptyFieldName)));
}

#[test]
fn rejects_array_with_zero_max_count() {
    let config = MulterConfig {
        selector: Selector::Array {
            name: "photos".to_owned(),
            max_count: Some(0),
        },
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidArrayMaxCount { .. })));
}

#[test]
fn rejects_empty_fields_selector() {
    let config = MulterConfig {
        selector: Selector::fields(Vec::<SelectedField>::new()),
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::EmptyFieldsSelector)));
}

#[test]
fn rejects_duplicate_field_names_in_fields_selector() {
    let config = MulterConfig {
        selector: Selector::fields([
            SelectedField::new("avatar"),
            SelectedField::new("avatar"),
        ]),
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::DuplicateFieldName { .. })));
}

#[test]
fn rejects_invalid_numeric_limit_values() {
    let mut limits = Limits::default();
    limits.max_files = Some(0);

    let config = MulterConfig {
        limits,
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidLimitValue { .. })));
}

#[test]
fn rejects_part_limit_greater_than_max_body_size() {
    let mut limits = Limits::default();
    limits.max_body_size = Some(8);
    limits.max_file_size = Some(16);

    let config = MulterConfig {
        limits,
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::LimitExceedsBodySize { .. })));
}

#[test]
fn rejects_invalid_mime_pattern() {
    let mut limits = Limits::default();
    limits.allowed_mime_types = vec!["image".to_owned()];

    let config = MulterConfig {
        limits,
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidMimePattern { .. })));
}

#[test]
fn builder_validation_surfaces_config_errors() {
    let config = MulterConfig {
        selector: Selector::single(""),
        unknown_field_policy: UnknownFieldPolicy::Ignore,
        ..MulterConfig::default()
    };

    let result = MulterBuilder::new().with_config(config).build_config();
    assert!(matches!(result, Err(ConfigError::EmptyFieldName)));
}
