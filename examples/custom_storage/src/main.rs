#![allow(missing_docs)]

use std::{collections::HashMap, io, sync::Arc};

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use bytes::Bytes;
use futures::StreamExt;
use multigear::{BoxStream, Multer, MulterError, StorageEngine, StorageError};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
struct HashMapStorage {
    files: Arc<RwLock<HashMap<String, Bytes>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HashMapKey(String);

impl HashMapStorage {
    async fn len(&self) -> usize {
        self.files.read().await.len()
    }
}

#[async_trait::async_trait]
impl StorageEngine for HashMapStorage {
    type Output = HashMapKey;
    type Error = StorageError;

    async fn store(
        &self,
        field_name: &str,
        _file_name: Option<&str>,
        _content_type: &str,
        mut stream: BoxStream<'_, Result<Bytes, MulterError>>,
    ) -> Result<Self::Output, Self::Error> {
        let key = format!("{field_name}-{}", self.files.read().await.len());
        let mut content = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|err| StorageError::new(err.to_string()))?;
            content.extend_from_slice(&chunk);
        }

        self.files
            .write()
            .await
            .insert(key.clone(), Bytes::from(content));
        Ok(HashMapKey(key))
    }
}

async fn upload(
    data: web::Data<Multer<HashMapStorage>>,
    request: HttpRequest,
    payload: web::Payload,
) -> impl Responder {
    let mut multipart = match data.parse(request, payload).await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };

    let mut stored_keys = Vec::new();
    while let Some(mut part) = match multipart.next_part().await {
        Ok(value) => value,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    } {
        if part.file_name().is_some() {
            match data.store(part).await {
                Ok(key) => stored_keys.push(key.0),
                Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
            };
        } else if let Err(err) = part.text().await {
            return HttpResponse::BadRequest().body(err.to_string());
        }
    }

    let total = data.storage().len().await;
    let mut body = format!(
        "stored {} file(s) in this request; custom storage now has {} item(s)\n",
        stored_keys.len(),
        total
    );
    for key in stored_keys {
        body.push_str(&format!("- key={key}\n"));
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
  <title>multigear custom storage upload</title>
</head>
<body>
  <h1>Custom Storage Example</h1>
  <p>Field name: <code>upload</code> (multiple allowed)</p>
  <form action="/upload" method="post" enctype="multipart/form-data">
    <input type="file" name="upload" multiple />
    <button type="submit">Upload</button>
  </form>
</body>
</html>
"#;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let storage = HashMapStorage::default();
    let multer = Multer::builder()
        .array("upload", 10)
        .storage(storage)
        .build()
        .expect("multer should build");

    let bind = ("127.0.0.1", 8088);
    let multer = web::Data::new(multer);
    println!(
        "custom-storage-example running at http://{}:{}",
        bind.0, bind.1
    );

    HttpServer::new(move || {
        App::new()
            .app_data(multer.clone())
            .route("/", web::get().to(index))
            .route("/upload", web::post().to(upload))
    })
    .bind(bind)?
    .run()
    .await
}
