#![allow(missing_docs)]

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

fn main() {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-axum-disk"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Arc::new(
        Multer::builder()
            .array("documents", 8)
            .max_file_size(16 * 1024 * 1024)
            .storage(storage)
            .build()
            .expect("multer should build"),
    );

    let _app: Router<()> = Router::new().route("/upload", post(upload)).with_state(multer);
}


