#![allow(missing_docs)]

use actix_web::{web, App, HttpRequest, HttpResponse, Responder};
use multigear::{DiskStorage, FilenameStrategy, Multer};

async fn upload(
    data: web::Data<Multer<DiskStorage>>,
    request: HttpRequest,
    payload: web::Payload,
) -> impl Responder {
    let mut multipart = match data.parse(request, payload).await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };

    while let Some(part) = match multipart.next_part().await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    } {
        if part.file_name().is_some() {
            if let Err(err) = data.store(part).await {
                return HttpResponse::BadRequest().body(err.to_string());
            }
        }
    }

    HttpResponse::Ok().finish()
}

fn main() {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-actix-disk"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Multer::builder()
        .array("files", 10)
        .max_file_size(64 * 1024 * 1024)
        .storage(storage)
        .build()
        .expect("multer should build");

    let _app = App::new()
        .app_data(web::Data::new(multer))
        .route("/upload", web::post().to(upload));
}


