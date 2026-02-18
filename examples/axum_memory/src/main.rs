#![allow(missing_docs)]

use std::{io, net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Router,
};
use multigear::{axum::MulterExtractor, MemoryStorage, Multer};

async fn upload(
    State(multer): State<Arc<Multer<MemoryStorage>>>,
    MulterExtractor(mut multipart): MulterExtractor,
) -> Result<String, (StatusCode, String)> {
    let mut stored = Vec::new();

    while let Some(part) = multipart.next_part().await.map_err(err)? {
        if part.file_name().is_some() {
            stored.push(multer.store(part).await.map_err(err)?);
        }
    }

    let total_files = multer.storage().len().await;
    let mut body = format!(
        "stored {} file(s) in this request; memory storage now has {} item(s)\n",
        stored.len(),
        total_files
    );
    for file in stored {
        let original_name = file.file_name.as_deref().unwrap_or("<none>");
        body.push_str(&format!(
            "- field={} original={} bytes={} key={}\n",
            file.field_name, original_name, file.size, file.storage_key
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
  <title>multigear axum memory upload</title>
</head>
<body>
  <h1>Axum Memory Upload Example</h1>
  <p>Field name: <code>avatar</code> (single file expected)</p>
  <form action="/upload/avatar" method="post" enctype="multipart/form-data">
    <input type="file" name="avatar" />
    <button type="submit">Upload</button>
  </form>
</body>
</html>
"#;

#[tokio::main]
async fn main() -> io::Result<()> {
    let multer = Arc::new(
        Multer::builder()
            .single("avatar")
            .storage(MemoryStorage::new())
            .build()
            .expect("multer should build"),
    );

    let app: Router<()> = Router::new()
        .route("/", get(index))
        .route("/upload/avatar", post(upload))
        .with_state(multer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8083));
    println!("axum-memory-example running at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}
