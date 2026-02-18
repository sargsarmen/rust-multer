use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::Stream;

use crate::{
    MulterError, ParseError,
    parser::headers::ParsedPartHeaders,
    parser::stream::ParsedPart,
};

/// Parsed multipart part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Parsed part headers.
    pub headers: ParsedPartHeaders,
    /// Raw part body bytes.
    pub body: Bytes,
    consumed: bool,
}

impl Part {
    /// Creates a high-level part from a low-level parsed part.
    pub(crate) fn from_parsed(parsed: ParsedPart) -> Self {
        Self {
            headers: parsed.headers,
            body: parsed.body,
            consumed: false,
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

    /// Returns parsed part headers.
    pub fn headers(&self) -> &ParsedPartHeaders {
        &self.headers
    }

    /// Returns the remaining body size hint in bytes.
    pub fn size_hint(&self) -> usize {
        if self.consumed { 0 } else { self.body.len() }
    }

    /// Reads the full part body as bytes.
    pub async fn bytes(&mut self) -> Result<Bytes, MulterError> {
        self.take_body()
    }

    /// Reads the full part body and decodes it as UTF-8 text.
    pub async fn text(&mut self) -> Result<String, MulterError> {
        let bytes = self.take_body()?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| ParseError::new("part body is not valid UTF-8").into())
    }

    /// Returns a one-shot body stream for this part.
    pub fn stream(&mut self) -> Result<PartBodyStream, MulterError> {
        Ok(PartBodyStream {
            body: Some(self.take_body()?),
        })
    }

    fn take_body(&mut self) -> Result<Bytes, MulterError> {
        if self.consumed {
            return Err(ParseError::new("part body was already consumed").into());
        }

        self.consumed = true;
        Ok(self.body.clone())
    }
}

/// One-shot stream returned by [`Part::stream`].
#[derive(Debug)]
pub struct PartBodyStream {
    body: Option<Bytes>,
}

impl Stream for PartBodyStream {
    type Item = Result<Bytes, MulterError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.body.take().map(Ok))
    }
}
