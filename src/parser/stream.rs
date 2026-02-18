use std::task::{Context, Poll};

use bytes::Bytes;
use futures::{Stream, future::poll_fn};
use http::{
    HeaderMap, HeaderName, HeaderValue,
    header::{self},
};

use crate::{
    MulterError, ParseError,
    parser::headers::{ParsedPartHeaders, parse_part_headers},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    StartBoundary,
    Headers,
    Body,
    End,
    Failed,
}

/// Stream-level limits enforced while parsing multipart input.
#[derive(Debug, Clone, Copy, Default)]
pub struct StreamLimits {
    /// Maximum accepted file size in bytes for a single file part.
    pub max_file_size: Option<u64>,
    /// Maximum accepted size in bytes for a text field.
    pub max_field_size: Option<u64>,
    /// Maximum request body size in bytes.
    pub max_body_size: Option<u64>,
}

/// Incremental multipart parser over a chunked byte stream.
#[derive(Debug)]
pub struct MultipartStream<S> {
    stream: S,
    boundary_line: Vec<u8>,
    boundary_end_line: Vec<u8>,
    delimiter: Vec<u8>,
    buffer: Vec<u8>,
    state: ParseState,
    current_headers: Option<ParsedPartHeaders>,
    current_part_max_size: Option<u64>,
    current_part_size: u64,
    current_part_is_file: bool,
    limits: StreamLimits,
    received_body_bytes: u64,
    upstream_done: bool,
}

impl<S> MultipartStream<S> {
    /// Creates a new streaming parser for a known multipart boundary.
    pub fn new(boundary: impl Into<String>, stream: S) -> Result<Self, ParseError> {
        Self::with_limits(boundary, stream, StreamLimits::default())
    }

    /// Creates a new streaming parser with explicit stream limits.
    pub fn with_limits(
        boundary: impl Into<String>,
        stream: S,
        limits: StreamLimits,
    ) -> Result<Self, ParseError> {
        let boundary = boundary.into();
        validate_boundary_input(&boundary)?;

        let boundary_line = format!("--{boundary}").into_bytes();
        let boundary_end_line = format!("--{boundary}--").into_bytes();
        let delimiter = format!("\r\n--{boundary}").into_bytes();

        Ok(Self {
            stream,
            boundary_line,
            boundary_end_line,
            delimiter,
            buffer: Vec::new(),
            state: ParseState::StartBoundary,
            current_headers: None,
            current_part_max_size: None,
            current_part_size: 0,
            current_part_is_file: false,
            limits,
            received_body_bytes: 0,
            upstream_done: false,
        })
    }

    /// Returns `true` when the parser is currently positioned in a part body.
    pub fn is_reading_part_body(&self) -> bool {
        self.state == ParseState::Body
    }

    /// Polls until the next part headers are available.
    pub fn poll_next_part_headers(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<ParsedPartHeaders>, MulterError>>
    where
        S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        loop {
            match self.state {
                ParseState::StartBoundary => {
                    let Some(line) = take_line(&mut self.buffer) else {
                        if self.upstream_done {
                            self.state = ParseState::Failed;
                            return Poll::Ready(Err(ParseError::new("missing opening boundary").into()));
                        }

                        match self.poll_fill_buffer(cx)? {
                            Poll::Ready(()) => continue,
                            Poll::Pending => return Poll::Pending,
                        }
                    };

                    if line == self.boundary_line {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("multipart parser: opening boundary detected");
                        self.state = ParseState::Headers;
                        continue;
                    }

                    if line == self.boundary_end_line {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("multipart parser: immediate terminal boundary detected");
                        self.state = ParseState::End;
                        continue;
                    }

                    #[cfg(feature = "tracing")]
                    tracing::warn!("multipart parser: malformed opening boundary");
                    self.state = ParseState::Failed;
                    return Poll::Ready(Err(ParseError::new("malformed opening boundary").into()));
                }
                ParseState::Headers => {
                    let Some(split) = find_subslice(&self.buffer, b"\r\n\r\n") else {
                        if self.upstream_done {
                            self.state = ParseState::Failed;
                            return Poll::Ready(Err(MulterError::IncompleteStream));
                        }

                        match self.poll_fill_buffer(cx)? {
                            Poll::Ready(()) => continue,
                            Poll::Pending => return Poll::Pending,
                        }
                    };

                    let raw = self.buffer[..split].to_vec();
                    self.buffer.drain(..split + 4);

                    let headers = match parse_header_block(&raw).and_then(|h| parse_part_headers(&h)) {
                        Ok(headers) => headers,
                        Err(err) => {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(error = %err, "multipart parser: failed to parse part headers");
                            self.state = ParseState::Failed;
                            return Poll::Ready(Err(err.into()));
                        }
                    };

                    self.current_part_is_file = headers.file_name.is_some();
                    self.current_part_max_size = if self.current_part_is_file {
                        self.limits.max_file_size
                    } else {
                        self.limits.max_field_size
                    };
                    self.current_part_size = 0;
                    self.current_headers = Some(headers.clone());
                    self.state = ParseState::Body;
                    #[cfg(feature = "tracing")]
                    tracing::trace!(
                        field_name = headers.field_name.as_str(),
                        file = headers.file_name.is_some(),
                        "multipart parser: part headers parsed"
                    );
                    return Poll::Ready(Ok(Some(headers)));
                }
                ParseState::Body => {
                    return Poll::Ready(Err(ParseError::new(
                        "previous part body must be consumed before requesting next part",
                    )
                    .into()));
                }
                ParseState::End => return Poll::Ready(Ok(None)),
                ParseState::Failed => return Poll::Ready(Ok(None)),
            }
        }
    }

    /// Polls the next chunk for the currently active part body.
    pub fn poll_next_part_chunk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<Bytes>, MulterError>>
    where
        S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        loop {
            if self.state != ParseState::Body {
                return Poll::Ready(Ok(None));
            }

            if let Some(split) = find_subslice(&self.buffer, &self.delimiter) {
                let suffix_start = split + self.delimiter.len();
                let Some(boundary_suffix) = self.buffer.get(suffix_start..) else {
                    if self.upstream_done {
                        self.state = ParseState::Failed;
                        return Poll::Ready(Err(MulterError::IncompleteStream));
                    }

                    match self.poll_fill_buffer(cx)? {
                        Poll::Ready(()) => continue,
                        Poll::Pending => return Poll::Pending,
                    }
                };

                let (consumed, is_terminal) = if boundary_suffix.starts_with(b"--\r\n") {
                    (suffix_start + 4, true)
                } else if boundary_suffix.starts_with(b"\r\n") {
                    (suffix_start + 2, false)
                } else if self.upstream_done && boundary_suffix == b"--" {
                    (suffix_start + 2, true)
                } else {
                    self.state = ParseState::Failed;
                    return Poll::Ready(Err(ParseError::new("malformed multipart boundary").into()));
                };

                if let Err(err) = self.ensure_part_limit(split as u64) {
                    self.state = ParseState::Failed;
                    return Poll::Ready(Err(err));
                }

                let emit_chunk = if split == 0 {
                    None
                } else {
                    let bytes = Bytes::copy_from_slice(&self.buffer[..split]);
                    self.current_part_size = self.current_part_size.saturating_add(split as u64);
                    Some(bytes)
                };

                self.buffer.drain(..consumed);
                self.current_headers = None;
                self.current_part_max_size = None;
                self.current_part_size = 0;
                self.current_part_is_file = false;
                self.state = if is_terminal {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("multipart parser: terminal boundary reached");
                    ParseState::End
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("multipart parser: moving to next part headers");
                    ParseState::Headers
                };

                return Poll::Ready(Ok(emit_chunk));
            }

            if has_malformed_boundary_line(
                &self.buffer,
                &self.boundary_line,
                &self.boundary_end_line,
            ) {
                #[cfg(feature = "tracing")]
                tracing::warn!("multipart parser: malformed boundary line detected");
                self.state = ParseState::Failed;
                return Poll::Ready(Err(ParseError::new("malformed multipart boundary").into()));
            }

            let max_tail = self.delimiter.len().saturating_sub(1);
            let safe_len = self.buffer.len().saturating_sub(max_tail);
            if safe_len > 0 {
                if let Err(err) = self.ensure_part_limit(safe_len as u64) {
                    self.state = ParseState::Failed;
                    return Poll::Ready(Err(err));
                }

                let bytes = Bytes::copy_from_slice(&self.buffer[..safe_len]);
                self.buffer.drain(..safe_len);
                self.current_part_size = self.current_part_size.saturating_add(safe_len as u64);
                return Poll::Ready(Ok(Some(bytes)));
            }

            if self.upstream_done {
                #[cfg(feature = "tracing")]
                tracing::warn!("multipart parser: upstream ended before terminal boundary");
                self.state = ParseState::Failed;
                return Poll::Ready(Err(MulterError::IncompleteStream));
            }

            match self.poll_fill_buffer(cx)? {
                Poll::Ready(()) => continue,
                Poll::Pending => return Poll::Pending,
            }
        }
    }

    /// Drains and discards the currently active part body, if any.
    pub async fn drain_current_part(&mut self) -> Result<(), MulterError>
    where
        S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        if !self.is_reading_part_body() {
            return Ok(());
        }

        loop {
            let next = poll_fn(|cx| self.poll_next_part_chunk(cx)).await?;
            if next.is_none() {
                return Ok(());
            }
        }
    }

    fn poll_fill_buffer(&mut self, cx: &mut Context<'_>) -> Result<Poll<()>, MulterError>
    where
        S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        match std::pin::Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Pending => Ok(Poll::Pending),
            Poll::Ready(Some(Ok(chunk))) => {
                if !chunk.is_empty() {
                    if let Some(max_body_size) = self.limits.max_body_size {
                        let next = self.received_body_bytes.saturating_add(chunk.len() as u64);
                        if next > max_body_size {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(
                                max_body_size = max_body_size,
                                received = next,
                                "multipart parser: body size limit exceeded"
                            );
                            self.state = ParseState::Failed;
                            return Err(MulterError::BodySizeLimitExceeded { max_body_size });
                        }
                        self.received_body_bytes = next;
                    }

                    self.buffer.extend_from_slice(&chunk);
                }
                Ok(Poll::Ready(()))
            }
            Poll::Ready(Some(Err(err))) => {
                self.state = ParseState::Failed;
                Err(err)
            }
            Poll::Ready(None) => {
                self.upstream_done = true;
                Ok(Poll::Ready(()))
            }
        }
    }

    fn ensure_part_limit(&self, additional: u64) -> Result<(), MulterError> {
        let Some(limit) = self.current_part_max_size else {
            return Ok(());
        };

        if self.current_part_size.saturating_add(additional) <= limit {
            return Ok(());
        }

        let field = self
            .current_headers
            .as_ref()
            .map(|headers| headers.field_name.clone())
            .unwrap_or_else(|| "<unknown>".to_owned());

        if self.current_part_is_file {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                field = field.as_str(),
                max_file_size = limit,
                "multipart parser: file size limit exceeded"
            );
            Err(MulterError::FileSizeLimitExceeded {
                field,
                max_file_size: limit,
            })
        } else {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                field = field.as_str(),
                max_field_size = limit,
                "multipart parser: field size limit exceeded"
            );
            Err(MulterError::FieldSizeLimitExceeded {
                field,
                max_field_size: limit,
            })
        }
    }
}

fn parse_header_block(raw: &[u8]) -> Result<HeaderMap, ParseError> {
    let text = std::str::from_utf8(raw).map_err(|_| ParseError::new("part headers must be UTF-8"))?;
    let mut headers = HeaderMap::new();

    for line in text.split("\r\n") {
        if line.is_empty() {
            continue;
        }

        let Some((raw_name, raw_value)) = line.split_once(':') else {
            return Err(ParseError::new("invalid part header line"));
        };

        let name = raw_name
            .trim()
            .parse::<HeaderName>()
            .map_err(|_| ParseError::new("invalid part header name"))?;
        let value = HeaderValue::from_str(raw_value.trim())
            .map_err(|_| ParseError::new("invalid part header value"))?;
        headers.append(name, value);
    }

    if !headers.contains_key(header::CONTENT_DISPOSITION) {
        return Err(ParseError::new("missing Content-Disposition header"));
    }

    Ok(headers)
}

fn take_line(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let split = find_subslice(buffer, b"\r\n")?;
    let line = buffer[..split].to_vec();
    buffer.drain(..split + 2);
    Some(line)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack.windows(needle.len()).position(|window| window == needle)
}

fn has_malformed_boundary_line(buffer: &[u8], boundary_line: &[u8], boundary_end_line: &[u8]) -> bool {
    let Some(prefix) = find_subslice(buffer, b"\r\n--") else {
        return false;
    };

    let line_start = prefix + 2;
    let Some(relative_end) = find_subslice(&buffer[line_start..], b"\r\n") else {
        return false;
    };
    let line = &buffer[line_start..line_start + relative_end];
    line != boundary_line && line != boundary_end_line
}

fn validate_boundary_input(boundary: &str) -> Result<(), ParseError> {
    if boundary.is_empty() {
        return Err(ParseError::new("multipart boundary cannot be empty"));
    }

    if boundary.contains('\r') || boundary.contains('\n') {
        return Err(ParseError::new("multipart boundary cannot contain CRLF"));
    }

    Ok(())
}

