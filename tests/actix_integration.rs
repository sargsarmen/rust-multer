#![allow(missing_docs)]

#[cfg(feature = "actix")]
use actix_web::{FromRequest, http::header, test, web};
#[cfg(feature = "actix")]
use rust_multer::{MemoryStorage, Multer, actix::MulterMiddleware};

#[cfg(feature = "actix")]
#[actix_web::test]
async fn parse_method_parses_actix_request_payload() {
    let body = concat!(
        "--BOUND\r\n",
        "Content-Disposition: form-data; name=\"field\"\r\n",
        "\r\n",
        "value\r\n",
        "--BOUND--\r\n"
    );
    let (request, payload) = test::TestRequest::default()
        .insert_header((header::CONTENT_TYPE, "multipart/form-data; boundary=BOUND"))
        .set_payload(body)
        .to_http_parts();
    let mut payload = payload;
    let payload = web::Payload::from_request(&request, &mut payload)
        .await
        .expect("payload extractor should succeed");
    let multer = Multer::new(MemoryStorage::new());

    let mut multipart = multer
        .parse(request, payload)
        .await
        .expect("parse should build multipart");
    let mut part = multipart
        .next_part()
        .await
        .expect("part parsing should succeed")
        .expect("part should exist");

    assert_eq!(part.field_name(), "field");
    assert_eq!(part.text().await.expect("text body should decode"), "value");
}

#[cfg(feature = "actix")]
#[actix_web::test]
async fn middleware_type_is_constructible() {
    let _middleware = MulterMiddleware;
}
