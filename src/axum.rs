//! Axum integration helpers.

use axum::{
    extract::FromRequest,
    body::Bytes,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use futures::{Stream, StreamExt, stream};
use std::pin::Pin;
use std::sync::Arc;

use crate::{Multer, MulterError, Multipart, ParseError, StorageEngine};

/// Axum body stream mapped into `multigear` chunk errors.
pub type AxumBodyStream<S> =
    stream::Map<S, fn(Result<Bytes, axum::Error>) -> Result<Bytes, MulterError>>;

/// Axum multipart type used by [`MulterExtractor`].
pub type AxumBodyBoxStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, MulterError>> + Send + 'static>>;

/// Axum multipart type used by [`MulterExtractor`].
pub type AxumMultipart = Multipart<AxumBodyBoxStream>;

/// Rejection type returned by Axum integration extractors.
#[derive(Debug)]
pub struct AxumMulterRejection(pub MulterError);

impl IntoResponse for AxumMulterRejection {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.0.to_string()).into_response()
    }
}

/// Trait implemented by Axum state types that can build `Multipart` via `Multer`.
pub trait MulterState {
    /// Builds multipart from content type and a streaming request body.
    fn build_multipart(
        &self,
        content_type: &str,
        body: AxumBodyBoxStream,
    ) -> Result<AxumMultipart, MulterError>;
}

impl<S> MulterState for Multer<S>
where
    S: StorageEngine,
{
    fn build_multipart(
        &self,
        content_type: &str,
        body: AxumBodyBoxStream,
    ) -> Result<AxumMultipart, MulterError> {
        self.multipart_from_content_type(content_type, body)
    }
}

impl<S> MulterState for Arc<Multer<S>>
where
    S: StorageEngine,
{
    fn build_multipart(
        &self,
        content_type: &str,
        body: AxumBodyBoxStream,
    ) -> Result<AxumMultipart, MulterError> {
        self.as_ref().build_multipart(content_type, body)
    }
}

/// Extractor that parses request body into [`Multipart`] using `Multer` state.
pub struct MulterExtractor(pub AxumMultipart);

impl std::fmt::Debug for MulterExtractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MulterExtractor").field(&"<multipart>").finish()
    }
}

#[async_trait::async_trait]
impl<AppState> FromRequest<AppState> for MulterExtractor
where
    AppState: Send + Sync + MulterState,
{
    type Rejection = AxumMulterRejection;

    async fn from_request(
        request: axum::extract::Request,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let (parts, body) = request.into_parts();
        let content_type = content_type_from_headers(&parts.headers).map_err(AxumMulterRejection)?;
        let body_stream = map_body_stream(body.into_data_stream());
        let body_stream = Box::pin(body_stream) as AxumBodyBoxStream;

        let multipart = state
            .build_multipart(content_type, body_stream)
            .map_err(AxumMulterRejection)?;

        Ok(Self(multipart))
    }
}

/// Extracts the raw `Content-Type` header from Axum request headers.
pub fn content_type_from_headers(headers: &HeaderMap) -> Result<&str, MulterError> {
    let value = headers
        .get(header::CONTENT_TYPE)
        .ok_or_else(|| ParseError::new("missing Content-Type header"))?;
    value
        .to_str()
        .map_err(|_| ParseError::new("Content-Type header must be ASCII").into())
}

/// Maps an Axum body stream into the stream shape expected by `multigear`.
pub fn map_body_stream<S>(stream: S) -> AxumBodyStream<S>
where
    S: Stream<Item = Result<Bytes, axum::Error>>,
{
    stream.map(axum_item_to_multer)
}

/// Creates a configured [`Multipart`] stream from Axum headers and body stream.
pub fn multipart_from_headers<S, B>(
    multer: &Multer<S>,
    headers: &HeaderMap,
    body: B,
) -> Result<Multipart<AxumBodyStream<B>>, MulterError>
where
    S: StorageEngine,
    B: Stream<Item = Result<Bytes, axum::Error>> + Unpin,
{
    let content_type = content_type_from_headers(headers)?;
    multer.multipart_from_content_type(content_type, map_body_stream(body))
}

fn axum_item_to_multer(item: Result<Bytes, axum::Error>) -> Result<Bytes, MulterError> {
    item.map_err(|err| ParseError::new(format!("axum body stream error: {err}")).into())
}

