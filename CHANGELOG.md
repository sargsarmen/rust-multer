# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Stream-based `StorageEngine` contract with backend-specific output support.
- `DiskStorage` builder parity with `destination(...)` and `filename(...)` APIs.
- Optional `DiskStorage` per-file `filter(...)` hook.
- `Multer::parse_stream(...)` for framework-agnostic `AsyncRead` ingestion.
- `Part::headers()` raw header map access and `Part::parsed_headers()` normalized metadata access.
- Axum `MulterExtractor` (`FromRequest`) integration surface.
- Actix `Multer::parse(...)`, `MulterData` extractor, and `MulterMiddleware`.
- Feature-gated integration tests for both Axum and Actix.
- Feature-gated `serde` derives for public configuration models (`Limits`, `MulterConfig`, selectors).
- Feature-gated `tracing` instrumentation across parser, limits, and storage hot paths.

### Changed
- `ProcessedMultipart` now supports backend-generic output while preserving built-in ergonomic defaults.
- `Part::stream()` now returns boxed stream surface for custom storage sinks.
- `Part::size_hint()` now reflects `Content-Length` header hints when present.
- README now includes 5-minute quickstarts for Axum and Actix.

### Security
- Expanded filename sanitization tests to cover traversal and null-byte inputs.

## [0.1.0] - 2026-02-18

### Added
- Streaming limit enforcement, including MIME allowlist validation.
- Storage abstraction with `MemoryStorage` and `DiskStorage`.
- `DiskStorage` filename strategies: `Keep`, `Random`, and `Custom`.
- Framework-agnostic end-to-end parse-and-store APIs.
- Optional Axum and Actix integration helpers behind feature flags.
- Examples, benchmark scaffold, and CI workflow gates.
