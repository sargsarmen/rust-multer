#![allow(missing_docs)]

use std::io;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
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

    let mut stored = Vec::new();
    let mut text_fields = Vec::new();

    while let Some(mut part) = match multipart.next_part().await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    } {
        if part.file_name().is_some() {
            match data.store(part).await {
                Ok(file) => stored.push(file),
                Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
            };
        } else {
            let field_name = part.field_name().to_owned();
            match part.text().await {
                Ok(text) => text_fields.push((field_name, text)),
                Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
            }
        }
    }

    let mut body = format!(
        "stored {} file(s), parsed {} text field(s)\n",
        stored.len(),
        text_fields.len()
    );

    for (name, value) in text_fields {
        body.push_str(&format!("- text field={name} value={value}\n"));
    }
    for file in stored {
        let original_name = file.file_name.as_deref().unwrap_or("<none>");
        let path = file
            .path
            .as_ref()
            .map(|value| value.display().to_string())
            .unwrap_or_else(|| "<none>".to_owned());
        body.push_str(&format!(
            "- file field={} original={} bytes={} path={}\n",
            file.field_name, original_name, file.size, path
        ));
    }

    HttpResponse::Ok().body(body)
}

async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(INDEX_HTML)
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>multigear actix fields upload</title>
</head>
<body>
  <h1>Actix Fields Example</h1>
  <p>Allowed fields: <code>metadata</code> (text), <code>thumbnail</code> (jpeg/png), <code>gallery</code> (image/*).</p>
  <form action="/products" method="post" enctype="multipart/form-data">
    <label>Metadata:</label><br />
    <textarea name="metadata" rows="4" cols="60">{"name":"sample product"}</textarea><br /><br />
    <label>Thumbnail:</label>
    <input type="file" name="thumbnail" accept="image/jpeg,image/png" /><br /><br />
    <label>Gallery:</label>
    <input type="file" name="gallery" accept="image/*" multiple /><br /><br />
    <button type="submit">Upload</button>
  </form>
</body>
</html>
"#;

#[actix_web::main]
async fn main() -> io::Result<()> {
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
            Field::file("gallery")
                .max_count(8)
                .allowed_mime_types(["image/*"]),
        ])
        .on_unknown_field(UnknownFieldPolicy::Reject)
        .storage(storage)
        .build()
        .expect("multer should build");

    let multer = web::Data::new(multer);
    let bind = ("127.0.0.1", 8082);
    println!(
        "actix-fields-example running at http://{}:{}",
        bind.0, bind.1
    );

    HttpServer::new(move || {
        App::new()
            .app_data(multer.clone())
            .route("/", web::get().to(index))
            .route("/products", web::post().to(upload))
    })
    .bind(bind)?
    .run()
    .await
}
