#![allow(missing_docs)]

use std::{convert::Infallible, io, net::SocketAddr, sync::Arc};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    body::Incoming,
    header,
    server::conn::http1,
    service::{service_fn, Service},
    Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use multigear::{hyper::MulterService, DiskStorage, FilenameStrategy, Multer, StoredFile};
use tokio::net::TcpListener;

async fn handle(
    request: Request<Incoming>,
    multer: Arc<Multer<DiskStorage>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Ok(html_response(INDEX_HTML)),
        (&Method::POST, "/upload") => {
            let service = MulterService::new(multer, |saved_files: Vec<StoredFile>| async move {
                Ok::<_, io::Error>(upload_response(saved_files))
            });

            match service.call(request).await {
                Ok(response) => Ok(response),
                Err(err) => Ok(text_response(
                    StatusCode::BAD_REQUEST,
                    format!("upload failed: {err}\n"),
                )),
            }
        }
        _ => Ok(text_response(StatusCode::NOT_FOUND, "not found\n")),
    }
}

fn upload_response(saved_files: Vec<StoredFile>) -> Response<Full<Bytes>> {
    let mut body = format!("stored {} file(s)\n", saved_files.len());
    for file in saved_files {
        let original_name = file.file_name.as_deref().unwrap_or("<none>");
        let path = file
            .path
            .as_ref()
            .map(|value| value.display().to_string())
            .unwrap_or_else(|| "<none>".to_owned());
        body.push_str(&format!(
            "- field={} original={} bytes={} path={}\n",
            file.field_name, original_name, file.size, path
        ));
    }

    text_response(StatusCode::OK, body)
}

fn text_response(status: StatusCode, body: impl Into<Bytes>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(body.into()))
        .expect("response should build")
}

fn html_response(body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("response should build")
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>multigear hyper service upload</title>
</head>
<body>
  <h1>Hyper MulterService Example</h1>
  <p>Field name: <code>files</code> (multiple allowed)</p>
  <form action="/upload" method="post" enctype="multipart/form-data">
    <input type="file" name="files" multiple />
    <button type="submit">Upload</button>
  </form>
</body>
</html>
"#;

#[tokio::main]
async fn main() -> io::Result<()> {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-hyper-service"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Arc::new(
        Multer::builder()
            .any()
            .storage(storage)
            .build()
            .expect("multer should build"),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 8087));
    let listener = TcpListener::bind(addr).await?;
    println!("hyper-service-example running at http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let multer = Arc::clone(&multer);

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |request| handle(request, Arc::clone(&multer)));

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("connection error: {err}");
            }
        });
    }
}
