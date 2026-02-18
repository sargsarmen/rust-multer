use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::Stream;

use crate::{
    MulterError, ParseError,
    Part,
    parser::stream::MultipartStream,
};

/// High-level multipart stream abstraction.
#[derive(Debug)]
pub struct Multipart<S> {
    inner: MultipartStream<S>,
}

impl<S> Multipart<S> {
    /// Creates a multipart stream from an already extracted boundary and a chunk source.
    pub fn new(boundary: impl Into<String>, stream: S) -> Result<Self, ParseError> {
        Ok(Self {
            inner: MultipartStream::new(boundary, stream)?,
        })
    }
}

impl<S> Stream for Multipart<S>
where
    S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
{
    type Item = Result<Part, MulterError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(parsed))) => Poll::Ready(Some(Ok(Part::from_parsed(parsed)))),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
