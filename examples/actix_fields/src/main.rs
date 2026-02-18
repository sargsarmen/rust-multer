#![allow(missing_docs)]

use actix_web::{web, App, HttpRequest, HttpResponse, Responder};
use multigear::{DiskStorage, Field, FilenameStrategy, Multer, UnknownFieldPolicy};

async fn upload(
    data: web::Data<Multer<DiskStorage>>,
    request: HttpRequest,
    payload: web::Payload,
) -> impl Responder {
    let mut multipart = match data.parse(request, payload).await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };

    while let Some(mut part) = match multipart.next_part().await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    } {
        if part.file_name().is_some() {
            if let Err(err) = data.store(part).await {
                return HttpResponse::BadRequest().body(err.to_string());
            }
        } else if let Err(err) = part.text().await {
            return HttpResponse::BadRequest().body(err.to_string());
        }
    }

    HttpResponse::Ok().finish()
}

fn main() {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-actix-fields"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Multer::builder()
        .fields([
            Field::text("metadata").max_size(16 * 1024),
            Field::file("thumbnail")
                .max_count(1)
                .allowed_mime_types(["image/jpeg", "image/png"]),
            Field::file("gallery").max_count(8).allowed_mime_types(["image/*"]),
        ])
        .on_unknown_field(UnknownFieldPolicy::Reject)
        .storage(storage)
        .build()
        .expect("multer should build");

    let _app = App::new()
        .app_data(web::Data::new(multer))
        .route("/products", web::post().to(upload));
}


