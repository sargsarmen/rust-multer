#![allow(missing_docs)]

#[cfg(feature = "axum")]
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::post,
};
#[cfg(feature = "axum")]
use std::sync::Arc;
#[cfg(feature = "axum")]
use rust_multer::{MemoryStorage, Multer, axum::MulterExtractor};

#[cfg(feature = "axum")]
async fn upload(
    State(multer): State<Arc<Multer<MemoryStorage>>>,
    MulterExtractor(mut multipart): MulterExtractor,
) -> Result<String, (StatusCode, String)> {
    let _ = multer;

    let mut count = 0usize;
    while multipart
        .next_part()
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
        .is_some()
    {
        count += 1;
    }

    Ok(format!("parsed {count} multipart parts"))
}

#[cfg(feature = "axum")]
fn main() {
    let multer = Arc::new(Multer::new(MemoryStorage::new()));
    let _app: Router<()> = Router::new()
        .route("/upload", post(upload))
        .with_state(multer);
}

#[cfg(not(feature = "axum"))]
fn main() {
    println!("Enable the `axum` feature to run this example.");
}

