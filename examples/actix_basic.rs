#![allow(missing_docs)]

#[cfg(feature = "actix")]
use actix_web::{App, HttpRequest, HttpResponse, Responder, web};
#[cfg(feature = "actix")]
use rust_multer::{MemoryStorage, Multer, actix::MulterMiddleware};

#[cfg(feature = "actix")]
async fn upload(
    data: web::Data<Multer<MemoryStorage>>,
    request: HttpRequest,
    payload: web::Payload,
) -> impl Responder {
    let mut multipart = match data.parse(request, payload).await
    {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };

    let mut count = 0usize;
    loop {
        match multipart.next_part().await {
            Ok(Some(_)) => count += 1,
            Ok(None) => break,
            Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
        }
    }

    HttpResponse::Ok().body(format!("parsed {count} multipart parts"))
}

#[cfg(feature = "actix")]
fn main() {
    let multer = Multer::new(MemoryStorage::new());
    let _app = App::new()
        .wrap(MulterMiddleware)
        .app_data(web::Data::new(multer))
        .route("/upload", web::post().to(upload));
}

#[cfg(not(feature = "actix"))]
fn main() {
    println!("Enable the `actix` feature to run this example.");
}

