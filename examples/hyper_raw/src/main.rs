#![allow(missing_docs)]

use std::{convert::Infallible, io, net::SocketAddr, sync::Arc};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{
    body::Incoming,
    header::{self, CONTENT_TYPE},
    server::conn::http1,
    service::service_fn,
    Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use multigear::{DiskStorage, FilenameStrategy, Multer};
use tokio::net::TcpListener;

async fn handle(
    request: Request<Incoming>,
    multer: Arc<Multer<DiskStorage>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => Ok(html_response(INDEX_HTML)),
        (&Method::POST, "/upload") => Ok(upload(request, multer).await),
        _ => Ok(text_response(StatusCode::NOT_FOUND, "not found\n")),
    }
}

async fn upload(
    request: Request<Incoming>,
    multer: Arc<Multer<DiskStorage>>,
) -> Response<Full<Bytes>> {
    let content_type = match request
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    {
        Some(value) => value,
        None => {
            return text_response(
                StatusCode::BAD_REQUEST,
                "missing or invalid Content-Type header\n",
            );
        }
    };
    let boundary = match multigear::extract_boundary(content_type) {
        Ok(value) => value,
        Err(err) => {
            return text_response(
                StatusCode::BAD_REQUEST,
                format!("invalid boundary: {err}\n"),
            )
        }
    };

    let stream = request.into_body().into_data_stream();
    let mut multipart = match multer.parse_stream(stream, boundary).await {
        Ok(value) => value,
        Err(err) => {
            return text_response(StatusCode::BAD_REQUEST, format!("parse failed: {err}\n"))
        }
    };

    let mut stored = Vec::new();
    while let Some(part) = match multipart.next_part().await {
        Ok(value) => value,
        Err(err) => {
            return text_response(
                StatusCode::BAD_REQUEST,
                format!("next part failed: {err}\n"),
            )
        }
    } {
        if part.file_name().is_some() {
            match multer.store(part).await {
                Ok(file) => stored.push(file),
                Err(err) => {
                    return text_response(
                        StatusCode::BAD_REQUEST,
                        format!("store failed: {err}\n"),
                    );
                }
            }
        }
    }

    let mut body = format!("stored {} file(s)\n", stored.len());
    for file in stored {
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
  <title>multigear hyper raw upload</title>
</head>
<body>
  <h1>Hyper Raw Upload Example</h1>
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
        .destination(std::env::temp_dir().join("multigear-hyper-raw"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Arc::new(
        Multer::builder()
            .array("files", 10)
            .storage(storage)
            .build()
            .expect("multer should build"),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 8086));
    let listener = TcpListener::bind(addr).await?;
    println!("hyper-raw-example running at http://{}", addr);

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
