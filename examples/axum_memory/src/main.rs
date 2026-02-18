#![allow(missing_docs)]

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Router};
use multigear::{axum::MulterExtractor, MemoryStorage, Multer};

async fn upload(
    State(multer): State<Arc<Multer<MemoryStorage>>>,
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

fn main() {
    let multer = Arc::new(
        Multer::builder()
            .single("avatar")
            .storage(MemoryStorage::new())
            .build()
            .expect("multer should build"),
    );

    let _app: Router<()> = Router::new()
        .route("/upload/avatar", post(upload))
        .with_state(multer);
}

