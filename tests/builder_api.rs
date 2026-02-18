#![allow(missing_docs)]

use multigear::{
    ConfigError, Field, Limits, Multer, MulterBuilder, MulterConfig, SelectedFieldKind, Selector,
    UnknownFieldPolicy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TestStorage {
    id: u8,
}

#[test]
fn builder_default_is_permissive() {
    let builder = MulterBuilder::default();
    assert_eq!(builder.config(), &MulterConfig::default());

    let multer = Multer::builder()
        .build()
        .expect("default builder config should be valid");
    assert_eq!(multer.config(), &MulterConfig::default());
}

#[test]
fn fluent_chaining_sets_expected_configuration() {
    let limits = Limits {
        max_file_size: Some(1024),
        max_files: Some(4),
        allowed_mime_types: vec!["image/*".to_owned()],
        ..Limits::default()
    };

    let multer = Multer::builder()
        .single("avatar")
        .unknown_field_policy(UnknownFieldPolicy::Reject)
        .limits(limits.clone())
        .build()
        .expect("builder config should validate");

    assert_eq!(
        multer.config(),
        &MulterConfig {
            selector: Selector::single("avatar"),
            unknown_field_policy: UnknownFieldPolicy::Reject,
            limits,
        }
    );
}

#[test]
fn builder_supports_custom_storage() {
    let multer = Multer::builder()
        .storage(TestStorage { id: 7 })
        .any()
        .build()
        .expect("builder config should validate");

    assert_eq!(multer.storage().id, 7);
}

#[test]
fn build_surfaces_config_errors() {
    let result = Multer::builder().array("photos", 0).build();
    assert!(matches!(
        result,
        Err(ConfigError::InvalidArrayMaxCount { .. })
    ));
}

#[test]
fn fluent_limit_shortcuts_set_expected_values() {
    let multer = Multer::builder()
        .max_file_size(10)
        .max_files(2)
        .max_field_size(20)
        .max_fields(3)
        .max_body_size(100)
        .allowed_mime_types(["image/*", "application/pdf"])
        .build()
        .expect("builder config should validate");

    assert_eq!(multer.config().limits.max_file_size, Some(10));
    assert_eq!(multer.config().limits.max_files, Some(2));
    assert_eq!(multer.config().limits.max_field_size, Some(20));
    assert_eq!(multer.config().limits.max_fields, Some(3));
    assert_eq!(multer.config().limits.max_body_size, Some(100));
    assert_eq!(
        multer.config().limits.allowed_mime_types,
        vec!["image/*".to_owned(), "application/pdf".to_owned()]
    );
}

#[test]
fn on_unknown_field_alias_matches_primary_api() {
    let multer = Multer::builder()
        .single("avatar")
        .on_unknown_field(UnknownFieldPolicy::Reject)
        .build()
        .expect("builder config should validate");

    assert_eq!(multer.config().unknown_field_policy, UnknownFieldPolicy::Reject);
}

#[test]
fn fields_accept_prd_style_field_descriptors() {
    let multer = Multer::builder()
        .fields([
            Field::new("avatar").max_count(1).allowed_mime_types(["image/*"]),
            Field::new("docs")
                .max_count(2)
                .allowed_mime_types(["application/pdf"]),
        ])
        .build()
        .expect("builder config should validate");

    match &multer.config().selector {
        Selector::Fields(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "avatar");
            assert_eq!(fields[0].kind, SelectedFieldKind::File);
            assert_eq!(fields[0].max_count, Some(1));
            assert_eq!(fields[0].max_size, None);
            assert_eq!(fields[0].allowed_mime_types, vec!["image/*".to_owned()]);
            assert_eq!(fields[1].name, "docs");
            assert_eq!(fields[1].kind, SelectedFieldKind::File);
            assert_eq!(fields[1].max_count, Some(2));
            assert_eq!(fields[1].max_size, None);
            assert_eq!(
                fields[1].allowed_mime_types,
                vec!["application/pdf".to_owned()]
            );
        }
        other => panic!("expected fields selector, got {other:?}"),
    }
}

#[test]
fn fields_support_file_and_text_models() {
    let multer = Multer::builder()
        .fields([
            Field::text("meta").max_size(8 * 1024),
            Field::file("avatar")
                .max_count(1)
                .allowed_mime_types(["image/png"]),
        ])
        .build()
        .expect("builder config should validate");

    match &multer.config().selector {
        Selector::Fields(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "meta");
            assert_eq!(fields[0].kind, SelectedFieldKind::Text);
            assert_eq!(fields[0].max_size, Some(8 * 1024));
            assert_eq!(fields[0].allowed_mime_types, Vec::<String>::new());

            assert_eq!(fields[1].name, "avatar");
            assert_eq!(fields[1].kind, SelectedFieldKind::File);
            assert_eq!(fields[1].max_count, Some(1));
            assert_eq!(fields[1].max_size, None);
            assert_eq!(fields[1].allowed_mime_types, vec!["image/png".to_owned()]);
        }
        other => panic!("expected fields selector, got {other:?}"),
    }
}

