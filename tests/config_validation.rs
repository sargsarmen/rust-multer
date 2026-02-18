#![allow(missing_docs)]

use multigear::{
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
    let limits = Limits {
        max_files: Some(0),
        ..Limits::default()
    };

    let config = MulterConfig {
        limits,
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidLimitValue { .. })));
}

#[test]
fn rejects_part_limit_greater_than_max_body_size() {
    let limits = Limits {
        max_body_size: Some(8),
        max_file_size: Some(16),
        ..Limits::default()
    };

    let config = MulterConfig {
        limits,
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::LimitExceedsBodySize { .. })));
}

#[test]
fn rejects_invalid_mime_pattern() {
    let limits = Limits {
        allowed_mime_types: vec!["image".to_owned()],
        ..Limits::default()
    };

    let config = MulterConfig {
        limits,
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidMimePattern { .. })));
}

#[test]
fn rejects_invalid_selected_field_mime_pattern() {
    let config = MulterConfig {
        selector: Selector::fields([
            SelectedField::new("avatar").allowed_mime_types(["image"]),
        ]),
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidMimePattern { .. })));
}

#[test]
fn rejects_invalid_selected_field_max_size() {
    let config = MulterConfig {
        selector: Selector::fields([SelectedField::text("meta").max_size(0)]),
        ..MulterConfig::default()
    };

    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidFieldMaxSize { .. })));
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

