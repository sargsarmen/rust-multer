#![allow(missing_docs)]

use std::{io, net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Router,
};
use multigear::{
    axum::MulterExtractor, DiskStorage, Field, FilenameStrategy, Multer, UnknownFieldPolicy,
};

async fn upload(
    State(multer): State<Arc<Multer<DiskStorage>>>,
    MulterExtractor(mut multipart): MulterExtractor,
) -> Result<String, (StatusCode, String)> {
    let mut stored = Vec::new();
    let mut text_fields = Vec::new();

    while let Some(mut part) = multipart.next_part().await.map_err(err)? {
        if part.file_name().is_some() {
            stored.push(multer.store(part).await.map_err(err)?);
        } else {
            let field_name = part.field_name().to_owned();
            let text = part.text().await.map_err(err)?;
            text_fields.push((field_name, text));
        }
    }

    let mut body = format!(
        "stored {} file(s), parsed {} text field(s)\n",
        stored.len(),
        text_fields.len()
    );

    for (name, value) in text_fields {
        body.push_str(&format!("- text field={} value={}\n", name, value));
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
  <title>multigear axum fields upload</title>
</head>
<body>
  <h1>Axum Fields Example</h1>
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let storage = DiskStorage::builder()
        .destination(std::env::temp_dir().join("multigear-axum-fields"))
        .filename(FilenameStrategy::Random)
        .build()
        .expect("disk storage should build");

    let multer = Arc::new(
        Multer::builder()
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
            .max_file_size(15 * 1024 * 1024)
            .storage(storage)
            .build()
            .expect("multer should build"),
    );

    let app: Router<()> = Router::new()
        .route("/", get(index))
        .route("/products", post(upload))
        .with_state(multer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8085));
    println!("axum-fields-example running at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}
