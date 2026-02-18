# rust-multer Execution Plan

This plan is derived from `rust-multer-prd.docx` and split into 13 independent implementation tasks.

## Ground Rules

- Execute tasks in order.
- Do not start the next task until the current task compile gate passes.
- Keep each task in a separate branch and PR.
- Keep public API additions backward compatible with previous completed tasks.
- At the end of each task, run `cargo check` at minimum.

## Suggested Branch Naming

- `task-01-bootstrap`
- `task-02-config-model`
- `task-03-error-model`
- `task-04-builder-api`
- `task-05-parse-primitives`
- `task-06-stream-parser`
- `task-07-part-api`
- `task-08-selector-engine`
- `task-09-limits-validation`
- `task-10-storage-memory`
- `task-11-storage-disk`
- `task-12-core-e2e-tests`
- `task-13-integrations-examples-ci`

## Target Layout

```text
src/
  lib.rs
  builder.rs
  config.rs
  error.rs
  field.rs
  limits.rs
  multipart.rs
  part.rs
  parser/
    mod.rs
    boundary.rs
    headers.rs
    stream.rs
  selector.rs
  storage/
    mod.rs
    memory.rs
    disk.rs
  axum.rs            # feature = "axum"
  actix.rs           # feature = "actix"
tests/
  parser_*.rs
  selector_*.rs
  limits_*.rs
  storage_*.rs
examples/
  axum_basic.rs
  actix_basic.rs
  custom_storage.rs
  streaming_large_file.rs
  field_validation.rs
benches/
  upload_bench.rs
```

## Task 01: Bootstrap Crate and Module Skeleton

### Files

- `Cargo.toml`
- `src/lib.rs`
- `src/builder.rs`
- `src/config.rs`
- `src/error.rs`
- `src/field.rs`
- `src/limits.rs`
- `src/multipart.rs`
- `src/part.rs`
- `src/parser/mod.rs`
- `src/storage/mod.rs`

### Checklist

- [ ] Rename crate to `rust-multer` (or `rust_multer` package name if needed).
- [ ] Add base dependencies (`bytes`, `futures`, `tokio`, `http`, `mime`, `thiserror`, `async-trait`, `uuid`, `pin-project`).
- [ ] Add feature flags from PRD (`axum`, `actix`, `tracing`, `serde`) with minimal wiring.
- [ ] Add empty module stubs and public re-exports from `lib.rs`.
- [ ] Enable strict lints and `#![warn(missing_docs)]` in `lib.rs`.

### Compile Gate

```bash
cargo check --all-targets
```

## Task 02: Configuration and Domain Model

### Files

- `src/config.rs`
- `src/field.rs`
- `src/limits.rs`
- `src/lib.rs`

### Checklist

- [ ] Implement `Field` model for file and text fields.
- [ ] Implement selectors: `single`, `array`, `fields`, `none`, `any`.
- [ ] Implement `UnknownFieldPolicy` (`Reject`, `Ignore`).
- [ ] Implement global limits model with defaults.
- [ ] Ensure all public types derive `Debug`.

### Compile Gate

```bash
cargo check --all-targets
```

## Task 03: Error Model and Config Validation

### Files

- `src/error.rs`
- `src/config.rs`
- `src/builder.rs`
- `tests/config_validation.rs`

### Checklist

- [ ] Implement `MulterError` as `#[non_exhaustive]`.
- [ ] Add config/build-time errors (`ConfigError`).
- [ ] Add conversion points for parser/storage failures.
- [ ] Validate conflicting selector configs and invalid limits.
- [ ] Add unit tests for invalid configuration cases.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test config_validation
```

## Task 04: Fluent Builder API

### Files

- `src/builder.rs`
- `src/lib.rs`
- `tests/builder_api.rs`

### Checklist

- [ ] Implement `Multer::builder()`.
- [ ] Implement fluent methods returning `Self`.
- [ ] Implement `.build() -> Result<Multer<S>, ConfigError>`.
- [ ] Ensure `MulterBuilder::default()` creates permissive dev config.
- [ ] Add API behavior tests for chaining and defaults.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test builder_api
```

## Task 05: Multipart Parse Primitives

### Files

- `src/parser/boundary.rs`
- `src/parser/headers.rs`
- `src/parser/mod.rs`
- `tests/parser_boundary.rs`
- `tests/parser_headers.rs`

### Checklist

- [ ] Extract and validate multipart boundary from `Content-Type`.
- [ ] Parse `Content-Disposition` (`name`, `filename`) robustly.
- [ ] Parse part `Content-Type` with `application/octet-stream` fallback.
- [ ] Handle quoted values and RFC-safe normalization.
- [ ] Add tests for malformed headers and malformed boundaries.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test parser_boundary --test parser_headers
```

## Task 06: Streaming Multipart State Machine

### Files

- `src/parser/stream.rs`
- `src/multipart.rs`
- `src/parser/mod.rs`
- `tests/parser_streaming.rs`

### Checklist

- [ ] Implement streaming parser that yields parts lazily.
- [ ] Avoid whole-body buffering.
- [ ] Detect malformed boundaries and incomplete terminal boundaries.
- [ ] Surface parse errors as structured `MulterError`.
- [ ] Add streaming tests with chunked input.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test parser_streaming
```

## Task 07: Part API Surface

### Files

- `src/part.rs`
- `src/multipart.rs`
- `tests/part_api.rs`

### Checklist

- [ ] Implement `field_name()`, `file_name()`, `content_type()`, `headers()`.
- [ ] Implement `bytes().await`, `text().await`, `stream()`, `size_hint()`.
- [ ] Enforce single-pass semantics where required.
- [ ] Add UTF-8 failure behavior in `text()` tests.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test part_api
```

## Task 08: Selector Engine and Field Matching

### Files

- `src/selector.rs`
- `src/config.rs`
- `src/multipart.rs`
- `tests/selector_rules.rs`

### Checklist

- [ ] Enforce `.single`, `.array`, `.fields`, `.none`, `.any`.
- [ ] Track per-field counts and reject excess counts.
- [ ] Apply `UnknownFieldPolicy`.
- [ ] Return `UnexpectedField` and count-limit errors consistently.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test selector_rules
```

## Task 09: Streaming Limits and MIME Validation

### Files

- `src/limits.rs`
- `src/multipart.rs`
- `src/selector.rs`
- `tests/limits_enforcement.rs`

### Checklist

- [ ] Enforce `max_file_size` during stream read.
- [ ] Enforce `max_files`, `max_field_size`, `max_fields`, `max_body_size`.
- [ ] Enforce `allowed_mime_types` with wildcard support (`image/*`).
- [ ] Ensure limit checks fail early before buffering large content.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test limits_enforcement
```

## Task 10: Storage Engine Trait and MemoryStorage

### Files

- `src/storage/mod.rs`
- `src/storage/memory.rs`
- `src/lib.rs`
- `tests/storage_memory.rs`

### Checklist

- [ ] Implement async `StorageEngine` trait.
- [ ] Define output metadata type for stored files.
- [ ] Implement `MemoryStorage`.
- [ ] Integrate `Multer::store(part)` for memory backend.
- [ ] Add storage conformance tests for memory backend.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test storage_memory
```

## Task 11: DiskStorage and Filename Sanitization

### Files

- `src/storage/disk.rs`
- `src/storage/mod.rs`
- `tests/storage_disk.rs`

### Checklist

- [ ] Implement `DiskStorage::builder()`.
- [ ] Implement filename strategy (`Keep`, `Random`, `Custom`).
- [ ] Sanitize filenames to block traversal and unsafe characters.
- [ ] Stream to disk with `tokio::fs` and low memory overhead.
- [ ] Return final path and size metadata.

### Compile Gate

```bash
cargo check --all-targets
cargo test --test storage_disk
```

## Task 12: Core End-to-End Paths and Conformance Tests

### Files

- `src/lib.rs`
- `src/multipart.rs`
- `tests/e2e_core.rs`
- `tests/storage_conformance.rs`

### Checklist

- [ ] Wire parser + selector + limits + storage in one execution path.
- [ ] Add framework-agnostic parse entry points.
- [ ] Add custom storage conformance tests (HashMap-like backend).
- [ ] Add regression tests for malformed stream and policy behavior.

### Compile Gate

```bash
cargo check --all-targets
cargo test --tests
```

## Task 13: Axum/Actix Integrations, Examples, Docs, CI Gates

### Files

- `src/axum.rs`
- `src/actix.rs`
- `examples/axum_basic.rs`
- `examples/actix_basic.rs`
- `examples/custom_storage.rs`
- `examples/streaming_large_file.rs`
- `examples/field_validation.rs`
- `README.md`
- `CHANGELOG.md`
- `.github/workflows/ci.yml`
- `benches/upload_bench.rs`

### Checklist

- [ ] Implement Axum integration behind `axum` feature.
- [ ] Implement Actix integration behind `actix` feature.
- [ ] Add runnable examples from PRD use cases.
- [ ] Add benchmark scaffold with Criterion.
- [ ] Add CI gates for check, test, clippy, docs.
- [ ] Ensure examples compile with feature flags.

### Compile Gate

```bash
cargo check --all-targets
cargo check --all-targets --features axum
cargo check --all-targets --features actix
cargo check --examples --all-features
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## Final Release Gate

Run this only after all tasks are complete:

```bash
cargo fmt --all --check
cargo check --all-targets --all-features
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## Optional Tracking Table

Use this table to track completion:

| Task | Status | Branch | PR | Compile Gate Passed |
|---|---|---|---|---|
| 01 | DONE | `task-01-bootstrap` | - | YES |
| 02 | DONE | `task-02-config-model` | - | YES |
| 03 | DONE | `task-03-error-model` | - | YES |
| 04 | DONE | `task-04-builder-api` | - | YES |
| 05 | DONE | `task-05-parse-primitives` | - | YES |
| 06 | DONE | `task-06-stream-parser` | - | YES |
| 07 | DONE | `task-07-part-api` | - | YES |
| 08 | TODO | `task-08-selector-engine` | - | No |
| 09 | TODO | `task-09-limits-validation` | - | No |
| 10 | TODO | `task-10-storage-memory` | - | No |
| 11 | TODO | `task-11-storage-disk` | - | No |
| 12 | TODO | `task-12-core-e2e-tests` | - | No |
| 13 | TODO | `task-13-integrations-examples-ci` | - | No |
