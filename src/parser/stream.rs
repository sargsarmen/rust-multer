use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::Stream;
use http::{
    HeaderMap, HeaderName, HeaderValue,
    header::{self},
};

use crate::{
    MulterError, ParseError,
    parser::headers::{ParsedPartHeaders, parse_part_headers},
};

/// Parsed multipart part produced by the streaming parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPart {
    /// Parsed part headers.
    pub headers: ParsedPartHeaders,
    /// Raw part body bytes.
    pub body: Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    StartBoundary,
    Headers,
    Body,
    End,
    Failed,
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
    upstream_done: bool,
}

impl<S> MultipartStream<S> {
    /// Creates a new streaming parser for a known multipart boundary.
    pub fn new(boundary: impl Into<String>, stream: S) -> Result<Self, ParseError> {
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
            upstream_done: false,
        })
    }
}

impl<S> Stream for MultipartStream<S>
where
    S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
{
    type Item = Result<ParsedPart, MulterError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.parse_available() {
                ParseOutcome::Emit(item) => return Poll::Ready(Some(item)),
                ParseOutcome::Done => return Poll::Ready(None),
                ParseOutcome::NeedMore => {}
            }

            if self.upstream_done {
                self.state = ParseState::Failed;
                return Poll::Ready(Some(Err(MulterError::IncompleteStream)));
            }

            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    if !chunk.is_empty() {
                        self.buffer.extend_from_slice(&chunk);
                    }
                }
                Poll::Ready(Some(Err(err))) => {
                    self.state = ParseState::Failed;
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Ready(None) => {
                    self.upstream_done = true;
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<S> MultipartStream<S> {
    fn parse_available(&mut self) -> ParseOutcome {
        loop {
            match self.state {
                ParseState::StartBoundary => {
                    let Some(line) = take_line(&mut self.buffer) else {
                        return if self.upstream_done {
                            ParseOutcome::Emit(Err(ParseError::new("missing opening boundary").into()))
                        } else {
                            ParseOutcome::NeedMore
                        };
                    };

                    if line == self.boundary_line {
                        self.state = ParseState::Headers;
                        continue;
                    }

                    if line == self.boundary_end_line {
                        self.state = ParseState::End;
                        continue;
                    }

                    self.state = ParseState::Failed;
                    return ParseOutcome::Emit(Err(ParseError::new("malformed opening boundary").into()));
                }
                ParseState::Headers => {
                    let Some(split) = find_subslice(&self.buffer, b"\r\n\r\n") else {
                        return ParseOutcome::NeedMore;
                    };

                    let raw = self.buffer[..split].to_vec();
                    self.buffer.drain(..split + 4);

                    let headers = match parse_header_block(&raw).and_then(|h| parse_part_headers(&h))
                    {
                        Ok(headers) => headers,
                        Err(err) => {
                            self.state = ParseState::Failed;
                            return ParseOutcome::Emit(Err(err.into()));
                        }
                    };

                    self.current_headers = Some(headers);
                    self.state = ParseState::Body;
                }
                ParseState::Body => {
                    let Some(split) = find_subslice(&self.buffer, &self.delimiter) else {
                        if has_malformed_boundary_line(
                            &self.buffer,
                            &self.boundary_line,
                            &self.boundary_end_line,
                        ) {
                            self.state = ParseState::Failed;
                            return ParseOutcome::Emit(Err(ParseError::new(
                                "malformed multipart boundary",
                            )
                            .into()));
                        }
                        return ParseOutcome::NeedMore;
                    };

                    let suffix_start = split + self.delimiter.len();
                    let Some(boundary_suffix) = self.buffer.get(suffix_start..) else {
                        return ParseOutcome::NeedMore;
                    };

                    let (consumed, is_terminal) = if boundary_suffix.starts_with(b"--\r\n") {
                        (suffix_start + 4, true)
                    } else if boundary_suffix.starts_with(b"\r\n") {
                        (suffix_start + 2, false)
                    } else if self.upstream_done && boundary_suffix == b"--" {
                        (suffix_start + 2, true)
                    } else {
                        self.state = ParseState::Failed;
                        return ParseOutcome::Emit(Err(ParseError::new(
                            "malformed multipart boundary",
                        )
                        .into()));
                    };

                    let body = Bytes::from(self.buffer[..split].to_vec());
                    self.buffer.drain(..consumed);

                    let Some(headers) = self.current_headers.take() else {
                        self.state = ParseState::Failed;
                        return ParseOutcome::Emit(Err(ParseError::new("missing part headers").into()));
                    };

                    self.state = if is_terminal {
                        ParseState::End
                    } else {
                        ParseState::Headers
                    };

                    return ParseOutcome::Emit(Ok(ParsedPart { headers, body }));
                }
                ParseState::End => return ParseOutcome::Done,
                ParseState::Failed => return ParseOutcome::Done,
            }
        }
    }
}

#[derive(Debug)]
enum ParseOutcome {
    NeedMore,
    Emit(Result<ParsedPart, MulterError>),
    Done,
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
