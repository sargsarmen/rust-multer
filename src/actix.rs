//! Actix integration helpers.

use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    task::{Context, Poll},
};

use actix_web::{
    FromRequest,
    HttpRequest,
    rt,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::PayloadError,
    http::header,
    web::{self, Bytes},
};
use futures::{Stream, StreamExt, channel::mpsc};

use crate::{Multer, MulterError, Multipart, ParseError, StorageEngine};

/// Actix body stream mapped into `multigear` chunk errors.
pub type ActixMappedBodyStream<S> =
    futures::stream::Map<S, fn(Result<Bytes, PayloadError>) -> Result<Bytes, MulterError>>;
/// Actix payload stream converted into a `Send` stream for multipart parsing.
pub type ActixBodyStream = mpsc::UnboundedReceiver<Result<Bytes, MulterError>>;

/// Extracts the raw `Content-Type` header from an Actix request.
pub fn content_type_from_request(request: &HttpRequest) -> Result<&str, MulterError> {
    let value = request
        .headers()
        .get(header::CONTENT_TYPE)
        .ok_or_else(|| ParseError::new("missing Content-Type header"))?;
    value
        .to_str()
        .map_err(|_| ParseError::new("Content-Type header must be ASCII").into())
}

/// Maps an Actix payload stream into the stream shape expected by `multigear`.
pub fn map_payload_stream<S>(stream: S) -> ActixMappedBodyStream<S>
where
    S: Stream<Item = Result<Bytes, PayloadError>>,
{
    stream.map(actix_item_to_multer)
}

fn payload_to_send_stream(payload: web::Payload) -> ActixBodyStream {
    let (tx, rx) = mpsc::unbounded::<Result<Bytes, MulterError>>();
    rt::spawn(async move {
        let mut stream = map_payload_stream(payload);
        while let Some(chunk) = stream.next().await {
            if tx.unbounded_send(chunk).is_err() {
                break;
            }
        }
    });
    rx
}

/// Creates a configured [`Multipart`] stream from an Actix request and payload stream.
pub fn multipart_from_request<S>(
    multer: &Multer<S>,
    request: &HttpRequest,
    payload: web::Payload,
) -> Result<Multipart<ActixBodyStream>, MulterError>
where
    S: StorageEngine,
{
    let content_type = content_type_from_request(request)?;
    multer.multipart_from_content_type(content_type, payload_to_send_stream(payload))
}

/// Helper that extracts multipart from an Actix request and payload.
pub fn extract_multipart<S>(
    multer: &Multer<S>,
    request: &HttpRequest,
    payload: web::Payload,
) -> Result<Multipart<ActixBodyStream>, MulterError>
where
    S: StorageEngine,
{
    multipart_from_request(multer, request, payload)
}

impl<S> Multer<S>
where
    S: StorageEngine,
{
    /// Parses an Actix request payload into a configured [`Multipart`] stream.
    pub async fn parse(
        &self,
        request: HttpRequest,
        payload: web::Payload,
    ) -> Result<Multipart<ActixBodyStream>, MulterError> {
        multipart_from_request(self, &request, payload)
    }
}

/// Actix extractor that provides `web::Data<Multer<S>>`.
#[derive(Debug)]
pub struct MulterData<S: StorageEngine>(pub web::Data<Multer<S>>);

impl<S> std::ops::Deref for MulterData<S>
where
    S: StorageEngine,
{
    type Target = Multer<S>;

    fn deref(&self) -> &Self::Target {
        self.0.get_ref()
    }
}

impl<S> Clone for MulterData<S>
where
    S: StorageEngine,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> FromRequest for MulterData<S>
where
    S: StorageEngine,
{
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut actix_web::dev::Payload) -> Self::Future {
        let fut = web::Data::<Multer<S>>::from_request(req, payload);
        Box::pin(async move { fut.await.map(Self) })
    }
}

/// Pass-through middleware marker for Multer-enabled Actix apps.
#[derive(Debug, Clone, Copy, Default)]
pub struct MulterMiddleware;

impl<T, B> Transform<T, ServiceRequest> for MulterMiddleware
where
    T: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = MulterMiddlewareService<T>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: T) -> Self::Future {
        ready(Ok(MulterMiddlewareService { service }))
    }
}

/// Service implementation for [`MulterMiddleware`].
#[derive(Debug)]
pub struct MulterMiddlewareService<T> {
    service: T,
}

impl<T, B> Service<ServiceRequest> for MulterMiddlewareService<T>
where
    T: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = T::Future;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, request: ServiceRequest) -> Self::Future {
        self.service.call(request)
    }
}

fn actix_item_to_multer(item: Result<Bytes, PayloadError>) -> Result<Bytes, MulterError> {
    item.map_err(|err| ParseError::new(format!("actix body stream error: {err}")).into())
}

