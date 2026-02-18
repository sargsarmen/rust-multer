#![allow(missing_docs)]

use actix_web::{web, App, HttpRequest, HttpResponse, Responder};
use multigear::{MemoryStorage, Multer};

async fn upload(
    data: web::Data<Multer<MemoryStorage>>,
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
    let multer = Multer::builder()
        .single("avatar")
        .storage(MemoryStorage::new())
        .build()
        .expect("multer should build");

    let _app = App::new()
        .app_data(web::Data::new(multer))
        .route("/upload/avatar", web::post().to(upload));
}

