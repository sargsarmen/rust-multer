<div align="center">

# multigear

Framework-agnostic multipart/form-data uploads for async Rust.

[![Crates.io](https://img.shields.io/crates/v/multigear.svg)](https://crates.io/crates/multigear)
[![Docs.rs](https://docs.rs/multigear/badge.svg)](https://docs.rs/multigear)
[![CI](https://github.com/sargsarmen/multigear/actions/workflows/ci.yml/badge.svg)](https://github.com/sargsarmen/multigear/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![MSRV: 1.75](https://img.shields.io/badge/rustc-1.75%2B-orange.svg)](https://blog.rust-lang.org/2023/12/28/Rust-1.75.0.html)

</div>

## What It Does

`multigear` provides a full upload pipeline on top of multipart parsing:
- selector rules: `.single()` `.array()` `.fields()` `.none()` `.any()`
- streaming limits: file, field, file-count, field-count, body-size
- MIME allowlists (global and per-field, with wildcard support)
- built-in storage engines: `MemoryStorage` and `DiskStorage`
- framework helpers for Axum, Actix-Web, and Hyper
- custom backend support via `StorageEngine`

## Install

```toml
[dependencies]
multigear = { version = "1", features = ["axum"] }
```

## Core Builder Example

```rust
use multigear::{DiskStorage, Field, FilenameStrategy, Multer, UnknownFieldPolicy};

let storage = DiskStorage::builder()
    .destination("/var/uploads")
    .filename(FilenameStrategy::Random)
    .build()?;

let multer = Multer::builder()
    .fields([
        Field::text("metadata").max_size(16 * 1024),
        Field::file("avatar")
            .max_count(1)
            .allowed_mime_types(["image/jpeg", "image/png"]),
        Field::file("documents")
            .max_count(5)
            .allowed_mime_types(["application/pdf"]),
    ])
    .on_unknown_field(UnknownFieldPolicy::Reject)
    .max_file_size(20 * 1024 * 1024)
    .max_body_size(80 * 1024 * 1024)
    .storage(storage)
    .build()?;
```

## Framework Support

### Axum (`features = ["axum"]`)

```rust
use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Router};
use multigear::{axum::MulterExtractor, DiskStorage, FilenameStrategy, Multer};

async fn upload(
    State(multer): State<Arc<Multer<DiskStorage>>>,
    MulterExtractor(mut multipart): MulterExtractor,
) -> Result<StatusCode, (StatusCode, String)> {
    while let Some(part) = multipart.next_part().await.map_err(err)? {
        if part.file_name().is_some() {
            multer.store(part).await.map_err(err)?;
        }
    }

    Ok(StatusCode::OK)
}

fn err(e: multigear::MulterError) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

# let multer = Arc::new(
#     Multer::builder()
#         .single("avatar")
#         .storage(
#             DiskStorage::builder()
#                 .destination(std::env::temp_dir().join("multigear-axum"))
#                 .filename(FilenameStrategy::Random)
#                 .build()
#                 .unwrap(),
#         )
#         .build()
#         .unwrap(),
# );
let _app: Router<()> = Router::new().route("/upload", post(upload)).with_state(multer);
```

### Actix-Web (`features = ["actix"]`)

```rust
use actix_web::{web, HttpRequest, HttpResponse};
use multigear::{DiskStorage, FilenameStrategy, Multer};

async fn upload(
    multer: web::Data<Multer<DiskStorage>>,
    req: HttpRequest,
    payload: web::Payload,
) -> HttpResponse {
    let mut multipart = match multer.parse(req, payload).await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };

    while let Some(part) = match multipart.next_part().await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    } {
        if part.file_name().is_some() {
            if let Err(err) = multer.store(part).await {
                return HttpResponse::BadRequest().body(err.to_string());
            }
        }
    }

    HttpResponse::Ok().finish()
}

# let _ = FilenameStrategy::Random;
```

### Hyper 1.0

Level 1 (no `hyper` feature): use `parse_stream` directly.

```rust
use http_body_util::BodyExt;
use multigear::{extract_boundary, MemoryStorage, Multer};

async fn parse_hyper_body(
    req: hyper::Request<hyper::body::Incoming>,
) -> Result<(), multigear::MulterError> {
    let multer = Multer::new(MemoryStorage::new());
    let content_type = req
        .headers()
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let boundary = extract_boundary(content_type)?;
    let stream = req.into_body().into_data_stream();
    let mut multipart = multer.parse_stream(stream, boundary).await?;

    while let Some(_part) = multipart.next_part().await? {}
    Ok(())
}
```

Level 2 (`features = ["hyper"]`): use `multigear::hyper::MulterService`.

## Storage Backends

### MemoryStorage

```rust
use multigear::{MemoryStorage, Multer};

# async fn demo(mut part: multigear::Part<'_>) -> Result<(), multigear::MulterError> {
let storage = MemoryStorage::new();
let multer = Multer::builder()
    .single("avatar")
    .storage(storage.clone())
    .build()?;

let saved = multer.store(part).await?;
let bytes = storage.get(&saved.storage_key).await;
# let _ = bytes;
# Ok(())
# }
```

### DiskStorage

```rust
use multigear::{DiskStorage, FilenameStrategy};

let storage = DiskStorage::builder()
    .destination("/var/uploads")
    .filename(FilenameStrategy::Random)
    // .filename(FilenameStrategy::Keep)
    // .filename(FilenameStrategy::Custom(|name| format!("safe-{}", name)))
    .build()?;
```

`DiskStorage` sanitizes output filenames before writing.

### Custom Storage

Implement `StorageEngine` and pass it to `.storage(...)`.

See: `examples/custom_storage/src/main.rs`.

## Feature Flags

| Flag | What it enables |
|---|---|
| `axum` | Axum extractor surface (`multigear::axum::MulterExtractor`) |
| `actix` | Actix helpers (`Multer::parse(req, payload)`, `MulterData`, middleware marker) |
| `hyper` | Hyper service wrapper (`multigear::hyper::MulterService`) |
| `tracing` | Structured tracing instrumentation across parser/limits/storage |
| `serde` | `Serialize`/`Deserialize` derives on public config models |
| `tokio-rt` (default) | Present as the default runtime feature marker; current behavior does not expose an independent runtime toggle |

## Examples

```bash
cargo run --example custom_storage
cargo run --example axum_memory --features axum
cargo run --example axum_disk --features axum
cargo run --example axum_fields --features axum
cargo run --example actix_memory --features actix
cargo run --example actix_disk --features actix
cargo run --example actix_fields --features actix
cargo run --example hyper_raw --features hyper
cargo run --example hyper_service --features hyper
```

## Development

```bash
cargo check --all-targets --all-features
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

## License

Licensed under either of:
- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
