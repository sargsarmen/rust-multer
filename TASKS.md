# rust-multer PRD Gap Execution Plan

This file tracks only the requirements still missing after comparing the current codebase with `docs/rust-multer-prd.docx`.

## How to Execute

- Execute tasks in order.
- Keep each task in a separate PR.
- Do not mark a task done until all checklist items and exit gates pass.

## Task 01 - Change Category: Package, Feature Flags, and Release Baseline

### Missing PRD Requirements

- `11.3 Feature Flags`: missing default `tokio-rt` feature behavior.
- `12 Cargo.toml Sketch`: metadata mismatch (`version`, `edition`, `rust-version`, `license`, `description`, `keywords`, `categories`).
- `14 Acceptance Criteria`: MSRV 1.88 support is not enforced.

### Required Changes

- [x] Add `tokio-rt` feature and make it part of `default`.
- [x] Align package metadata with PRD release expectations.
- [x] Set and enforce `rust-version = "1.88"`.
- [x] Ensure dependencies/features remain compatible with MSRV gate.

### Exit Gate

```bash
cargo check --all-targets --all-features
cargo +1.88.0 check --all-targets --all-features
```

## Task 02 - Change Category: Streaming Parser and Memory-Safety Compliance

### Missing PRD Requirements

- `F-PARSE-02`, `F-PARSE-06`: parser currently buffers full part bodies before yielding.
- `F-LIMIT-07`, `9 Limit Enforcement`: limits must be enforced while streaming without full-file buffering.
- `10 Performance Requirements`: current architecture does not guarantee low-RSS streaming path.

### Required Changes

- [x] Redesign parser/part pipeline to stream file bytes incrementally (no full-part buffering).
- [x] Ensure file and body limits are enforced during chunk flow, before full payload accumulation.
- [x] Ensure disk storage can consume streamed chunks directly from parser.
- [x] Add large-file streaming tests to prove no OOM behavior on constrained memory profiles.

### Exit Gate

```bash
cargo test --test parser_streaming --test limits_enforcement --test storage_disk
cargo run --example streaming_large_file
```

## Task 03 - Change Category: RFC 7578 Edge-Case Parsing

### Missing PRD Requirements

- `F-PARSE-05`: percent-encoded boundary handling is missing.
- `F-PARSE-05`: filename percent-encoding support must be complete (including edge-case coverage).

### Required Changes

- [x] Support percent-encoded boundary parsing and normalization.
- [x] Expand filename percent-encoding support/tests for RFC-compliant behavior.
- [x] Add malformed percent-encoding rejection tests for boundary/filename paths.

### Exit Gate

```bash
cargo test --test parser_boundary --test parser_headers
```

## Task 04 - Change Category: Builder API and Validation Model Parity

### Missing PRD Requirements

- `5.6 Fluent Builder API`: missing builder shortcuts (`max_file_size`, `max_files`, `max_field_size`, `max_fields`, `max_body_size`, `allowed_mime_types`).
- `5.3 Limits & Validation`: per-field validation model in examples is not fully represented by public API.
- `11.1 API Ergonomics`: PRD method surface differs from current builder naming and configuration flow.

### Required Changes

- [ ] Add fluent limit/mime methods directly on `MulterBuilder`.
- [ ] Add builder method parity for unknown field behavior (`on_unknown_field` alias/primary API).
- [ ] Reconcile `Field` model with selector/validation API used in PRD examples.
- [ ] Add tests for all fluent methods and mixed per-field/global validation behavior.

### Exit Gate

```bash
cargo test --test builder_api --test config_validation --test selector_rules --test limits_enforcement
```

## Task 05 - Change Category: Storage Trait and Backend Extensibility

### Missing PRD Requirements

- `5.5 Storage Trait`: current trait signature does not match stream-based extensibility contract.
- `5.4.2 Disk Storage`: missing optional per-file filter closure.
- `5.4.2 Disk Storage`: builder naming/API differs (`destination`, `filename` expected).

### Required Changes

- [ ] Refactor `StorageEngine` to stream-based store contract compatible with third-party backends.
- [ ] Support backend-specific output type while preserving ergonomic defaults.
- [ ] Add `DiskStorage` filter hook and PRD-aligned builder naming (`destination`, `filename`).
- [ ] Keep filename sanitization secure-by-default and test traversal/null-byte cases.

### Exit Gate

```bash
cargo test --test storage_conformance --test storage_memory --test storage_disk
cargo run --example custom_storage
```

## Task 06 - Change Category: Part and Multipart Public API Parity

### Missing PRD Requirements

- `7 Part & Field API`: `headers()` should expose header map equivalent.
- `7 Part & Field API`: `stream()` should be zero-copy oriented stream surface.
- `7 Part & Field API`: `size_hint()` should reflect header/stream hint semantics.
- `6.3 Framework-Agnostic Core`: expected `parse_stream(...)` API is missing.

### Required Changes

- [ ] Add `Multipart::next_part()` ergonomic API (retain Stream impl compatibility).
- [ ] Add/align `Multer::parse_stream(...)` for framework-agnostic ingestion.
- [ ] Align `Part::headers()`, `Part::stream()`, and `Part::size_hint()` with PRD contract.
- [ ] Add focused API tests for single-pass semantics and metadata fidelity.

### Exit Gate

```bash
cargo test --test part_api --test e2e_core
```

## Task 07 - Change Category: Axum and Actix First-Class Integrations

### Missing PRD Requirements

- `6.1 Axum`: no `MulterExtractor` implementing `FromRequest`.
- `6.2 Actix-Web`: no middleware/data extractor surface matching PRD flow.

### Required Changes

- [ ] Implement Axum extractor type(s) per PRD integration flow.
- [ ] Implement Actix integration helpers/extractor/middleware surface per PRD flow.
- [ ] Ensure integration APIs compose cleanly with `Multer<S>` state and `store(part)` workflow.
- [ ] Add integration tests/doc examples for both frameworks.

### Exit Gate

```bash
cargo check --all-targets --features axum
cargo check --all-targets --features actix
cargo run --example axum_basic --features axum
cargo run --example actix_basic --features actix
```

## Task 08 - Change Category: Tracing, Serde, and Developer Experience

### Missing PRD Requirements

- `11.3 Feature Flags`: `tracing` and `serde` flags exist but are not fully wired to behavior/derives.
- `11.1 API Ergonomics`: rustdoc runnable example coverage is incomplete.
- `11.2 Examples & Documentation`: README lacks 5-minute quickstart for both Axum and Actix.
- `11.2 Examples & Documentation`: changelog is not in Keep a Changelog structure.

### Required Changes

- [ ] Add `serde` derives behind feature flags on public configuration models.
- [ ] Add meaningful `tracing` instrumentation in parser/limits/storage hot paths.
- [ ] Expand rustdoc examples on core public APIs and verify compilation.
- [ ] Rewrite README quickstart sections for Axum and Actix.
- [ ] Reformat `CHANGELOG.md` to Keep a Changelog format.

### Exit Gate

```bash
cargo test --doc --all-features
cargo check --all-features
```

## Task 09 - Change Category: CI, Security, Performance, and Release Acceptance

### Missing PRD Requirements

- `10 Performance Requirements`: no automated acceptance gate for large-file memory behavior.
- `14 Acceptance Criteria`: CI is Ubuntu-only; no macOS/Windows matrix.
- `14 Acceptance Criteria`: no MSRV lane, no `cargo audit` gate, no benchmark regression run in CI.

### Required Changes

- [ ] Expand CI matrix to Linux/macOS/Windows.
- [ ] Add dedicated MSRV 1.88 job.
- [ ] Add `cargo audit` job (install `cargo-audit` in CI).
- [ ] Add benchmark CI job and threshold/regression policy.
- [ ] Add stress/integration scenario for multi-GB DiskStorage uploads and memory assertions.

### Exit Gate

```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
cargo bench --bench upload_bench
```

## Tracking

| Task | Category | Status |
|---|---|---|
| 01 | Package/Feature/Release Baseline | DONE |
| 02 | Streaming Parser and Memory-Safety | DONE |
| 03 | RFC 7578 Edge Cases | DONE |
| 04 | Builder API and Validation Model | TODO |
| 05 | Storage Trait and Backend Extensibility | TODO |
| 06 | Part/Multipart Public API Parity | TODO |
| 07 | Framework Integrations | TODO |
| 08 | Tracing/Serde/DX Docs | TODO |
| 09 | CI/Security/Performance Acceptance | TODO |
