#![allow(missing_docs)]

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Router};
use multigear::{
    axum::MulterExtractor, DiskStorage, Field, FilenameStrategy, Multer, UnknownFieldPolicy,
};

async fn upload(
    State(multer): State<Arc<Multer<DiskStorage>>>,
    MulterExtractor(mut multipart): MulterExtractor,
) -> Result<StatusCode, (StatusCode, String)> {
    while let Some(mut part) = multipart.next_part().await.map_err(err)? {
        if part.file_name().is_some() {
            multer.store(part).await.map_err(err)?;
        } else {
            let _text = part.text().await.map_err(err)?;
        }
    }

    Ok(StatusCode::OK)
}

fn err(e: multigear::MulterError) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

fn main() {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-axum-fields"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Arc::new(
        Multer::builder()
            .fields([
                Field::text("metadata").max_size(16 * 1024),
                Field::file("thumbnail")
                    .max_count(1)
                    .allowed_mime_types(["image/jpeg", "image/png"]),
                Field::file("gallery").max_count(8).allowed_mime_types(["image/*"]),
            ])
            .on_unknown_field(UnknownFieldPolicy::Reject)
            .max_file_size(15 * 1024 * 1024)
            .storage(storage)
            .build()
            .expect("multer should build"),
    );

    let _app: Router<()> = Router::new().route("/products", post(upload)).with_state(multer);
}


