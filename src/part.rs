use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{Stream, StreamExt};
use http::{HeaderMap, header};

use crate::{BoxStream, MulterError, ParseError, parser::headers::ParsedPartHeaders};

pub(crate) trait PartBodyReader {
    fn poll_next_chunk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<Bytes>, MulterError>>;
}

/// Parsed multipart part.
pub struct Part<'a> {
    /// Parsed part headers.
    pub headers: ParsedPartHeaders,
    body_reader: Option<&'a mut dyn PartBodyReader>,
}

impl fmt::Debug for Part<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Part")
            .field("headers", &self.headers)
            .field("consumed", &self.body_reader.is_none())
            .finish()
    }
}

impl<'a> Part<'a> {
    /// Creates a high-level part from parsed headers and a body reader.
    pub(crate) fn new(
        headers: ParsedPartHeaders,
        body_reader: &'a mut dyn PartBodyReader,
    ) -> Self {
        Self {
            headers,
            body_reader: Some(body_reader),
        }
    }

    /// Returns the logical field name for this part.
    pub fn field_name(&self) -> &str {
        &self.headers.field_name
    }

    /// Returns the optional file name for this part.
    pub fn file_name(&self) -> Option<&str> {
        self.headers.file_name.as_deref()
    }

    /// Returns the parsed content type for this part.
    pub fn content_type(&self) -> &mime::Mime {
        &self.headers.content_type
    }

    /// Returns raw part headers.
    ///
    /// `headers()` exposes the original map for advanced inspection, while
    /// [`Part::parsed_headers`] provides the normalized view.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers.headers
    }

    /// Returns parsed part headers.
    pub fn parsed_headers(&self) -> &ParsedPartHeaders {
        &self.headers
    }

    /// Returns the approximate body size hint in bytes from `Content-Length`, when present.
    ///
    /// The hint may be `None` when the incoming part does not declare a
    /// `Content-Length` header.
    pub fn size_hint(&self) -> Option<u64> {
        self.headers
            .headers
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
    }

    /// Reads the full part body as bytes.
    pub async fn bytes(&mut self) -> Result<Bytes, MulterError> {
        let mut stream = self.stream()?;
        let mut out = Vec::new();
        while let Some(chunk) = stream.next().await {
            out.extend_from_slice(&chunk?);
        }
        Ok(Bytes::from(out))
    }

    /// Reads the full part body and decodes it as UTF-8 text.
    pub async fn text(&mut self) -> Result<String, MulterError> {
        let bytes = self.bytes().await?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| ParseError::new("part body is not valid UTF-8").into())
    }

    /// Returns a one-shot body stream for this part.
    ///
    /// The returned stream can only be created once; subsequent calls return an
    /// "already consumed" error.
    pub fn stream(&mut self) -> Result<BoxStream<'_, Result<Bytes, MulterError>>, MulterError> {
        let Some(body_reader) = self.body_reader.take() else {
            return Err(ParseError::new("part body was already consumed").into());
        };

        Ok(Box::pin(PartBodyStream {
            body_reader,
            finished: false,
        }))
    }
}

/// One-shot stream returned by [`Part::stream`].
pub struct PartBodyStream<'a> {
    body_reader: &'a mut dyn PartBodyReader,
    finished: bool,
}

impl fmt::Debug for PartBodyStream<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PartBodyStream")
            .field("finished", &self.finished)
            .finish()
    }
}

impl Stream for PartBodyStream<'_> {
    type Item = Result<Bytes, MulterError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }

        match self.body_reader.poll_next_chunk(cx) {
            Poll::Ready(Ok(Some(bytes))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(Ok(None)) => {
                self.finished = true;
                Poll::Ready(None)
            }
            Poll::Ready(Err(err)) => {
                self.finished = true;
                Poll::Ready(Some(Err(err)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

