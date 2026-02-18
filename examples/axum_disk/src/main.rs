#![allow(missing_docs)]

use std::{io, net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Router,
};
use multigear::{axum::MulterExtractor, DiskStorage, FilenameStrategy, Multer};

async fn upload(
    State(multer): State<Arc<Multer<DiskStorage>>>,
    MulterExtractor(mut multipart): MulterExtractor,
) -> Result<String, (StatusCode, String)> {
    let mut stored = Vec::new();

    while let Some(part) = multipart.next_part().await.map_err(err)? {
        if part.file_name().is_some() {
            stored.push(multer.store(part).await.map_err(err)?);
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

    Ok(body)
}

fn err(e: multigear::MulterError) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>multigear axum disk upload</title>
</head>
<body>
  <h1>Axum Disk Upload Example</h1>
  <p>Field name: <code>documents</code> (up to 8 files)</p>
  <form action="/upload" method="post" enctype="multipart/form-data">
    <input type="file" name="documents" multiple />
    <button type="submit">Upload</button>
  </form>
</body>
</html>
"#;

#[tokio::main]
async fn main() -> io::Result<()> {
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

    let app: Router<()> = Router::new()
        .route("/", get(index))
        .route("/upload", post(upload))
        .with_state(multer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8084));
    println!("axum-disk-example running at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}
