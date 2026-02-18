# rust-multer

`rust-multer` is a streaming multipart/form-data parser with selector rules, request limits, and pluggable storage engines.

## Features

- Streaming parser with structured errors
- Selector engine: `single`, `array`, `fields`, `none`, `any`
- Limits: file size, field size, file count, field count, body size, MIME allowlist
- Storage engines:
  - `MemoryStorage`
  - `DiskStorage` (with filename sanitization and strategy controls)
- Optional framework helpers:
  - `axum` feature
  - `actix` feature

## Quick Start

### Core (Framework-Agnostic)

```rust
use bytes::Bytes;
use futures::stream;
use rust_multer::{MemoryStorage, Multer, MulterError};

#[tokio::main]
async fn main() {
    let multer = Multer::new(MemoryStorage::new());
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n",
        "\r\n",
        "hello\r\n",
        "--BOUND--\r\n"
    );

    let output = multer
        .parse_and_store(
            "BOUND",
            stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]),
        )
        .await
        .expect("multipart parse");

    println!("stored files: {}", output.stored_files.len());
}
```

### Axum (5-Minute Upload Handler)

Add dependencies:

```toml
rust-multer = { path = ".", features = ["axum"] }
axum = "0.8"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Handler flow:

```rust
use std::sync::Arc;
use axum::{extract::State, http::StatusCode};
use rust_multer::{DiskStorage, Multer, axum::MulterExtractor};

async fn upload(
    State(multer): State<Arc<Multer<DiskStorage>>>,
    MulterExtractor(mut multipart): MulterExtractor,
) -> Result<StatusCode, (StatusCode, String)> {
    while let Some(part) = multipart.next_part().await.map_err(err)? {
        multer.store(part).await.map_err(err)?;
    }
    Ok(StatusCode::OK)
}

fn err(e: rust_multer::MulterError) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}
```

### Actix-Web (5-Minute Upload Handler)

Add dependencies:

```toml
rust-multer = { path = ".", features = ["actix"] }
actix-web = "4"
```

Handler flow:

```rust
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use rust_multer::{DiskStorage, Multer};

async fn upload(
    data: web::Data<Multer<DiskStorage>>,
    req: HttpRequest,
    payload: web::Payload,
) -> impl Responder {
    let mut multipart = match data.parse(req, payload).await {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    while let Some(part) = match multipart.next_part().await {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    } {
        if let Err(e) = data.store(part).await {
            return HttpResponse::BadRequest().body(e.to_string());
        }
    }

    HttpResponse::Ok().finish()
}
```

## Examples

- `cargo run --example custom_storage`
- `cargo run --example streaming_large_file`
- `cargo run --example field_validation`
- `cargo run --example axum_basic --features axum`
- `cargo run --example actix_basic --features actix`

## Development

```bash
cargo check --all-targets --all-features
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```
