//! Hyper integration helpers.

use std::{future::Future, pin::Pin, sync::Arc};

use bytes::Bytes;
use futures::{Stream, StreamExt};
use http_body_util::BodyExt;
use hyper::{header, service::Service, Request, Response};

use crate::{parser, Multer, MulterError, ParseError, StorageEngine};

/// Boxed error type used by [`MulterService`].
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
/// Hyper body stream mapped into `multigear` chunk errors.
pub type HyperBodyBoxStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, MulterError>> + Send + 'static>>;

/// Service wrapper that parses multipart requests and forwards stored files to a handler.
#[derive(Clone)]
pub struct MulterService<S, H> {
    multer: Arc<Multer<S>>,
    handler: H,
}

impl<S, H> std::fmt::Debug for MulterService<S, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MulterService")
            .field("multer", &"<multer>")
            .field("handler", &"<fn>")
            .finish()
    }
}

impl<S, H> MulterService<S, H> {
    /// Creates a new Hyper service wrapper around a configured multer instance.
    pub fn new(multer: Arc<Multer<S>>, handler: H) -> Self {
        Self { multer, handler }
    }
}

impl<S, H, ReqBody, ResBody, Fut, E> Service<Request<ReqBody>> for MulterService<S, H>
where
    S: StorageEngine,
    ReqBody: hyper::body::Body<Data = Bytes> + Send + 'static,
    ReqBody::Error: std::error::Error + Send + Sync + 'static,
    H: Fn(Vec<S::Output>) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<Response<ResBody>, E>> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    type Response = Response<ResBody>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<ReqBody>) -> Self::Future {
        let multer = Arc::clone(&self.multer);
        let handler = self.handler.clone();

        Box::pin(async move {
            let content_type = content_type_from_request(&request).map_err(into_box_error)?;
            let boundary =
                parser::extract_multipart_boundary(content_type).map_err(into_box_error)?;
            let body_stream = map_body_stream(request.into_body());

            let mut multipart = multer
                .parse_stream(body_stream, boundary)
                .await
                .map_err(into_box_error)?;

            let mut saved_files = Vec::new();
            while let Some(part) = multipart.next_part().await.map_err(into_box_error)? {
                if part.file_name().is_some() {
                    let stored = multer.store(part).await.map_err(into_box_error)?;
                    saved_files.push(stored);
                }
            }

            handler(saved_files).await.map_err(into_box_error)
        })
    }
}

/// Extracts the raw `Content-Type` header from a Hyper request.
pub fn content_type_from_request<B>(request: &Request<B>) -> Result<&str, MulterError> {
    let value = request
        .headers()
        .get(header::CONTENT_TYPE)
        .ok_or_else(|| ParseError::new("missing Content-Type header"))?;
    value
        .to_str()
        .map_err(|_| ParseError::new("Content-Type header must be ASCII").into())
}

/// Maps a Hyper body into the stream shape expected by `multigear`.
pub fn map_body_stream<B>(body: B) -> HyperBodyBoxStream
where
    B: hyper::body::Body<Data = Bytes> + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let stream = body
        .into_data_stream()
        .map(hyper_item_to_multer::<B::Error>);
    Box::pin(stream)
}

fn hyper_item_to_multer<E>(item: Result<Bytes, E>) -> Result<Bytes, MulterError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    item.map_err(|err| ParseError::new(format!("hyper body stream error: {err}")).into())
}

fn into_box_error<E>(err: E) -> BoxError
where
    E: std::error::Error + Send + Sync + 'static,
{
    Box::new(err)
}

