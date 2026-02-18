use std::task::{Context, Poll};

use bytes::Bytes;
use futures::{Stream, future::poll_fn};

use crate::{
    Limits, MulterConfig, MulterError, ParseError, Selector, UnknownFieldPolicy,
    Part,
    parser::stream::{MultipartStream, StreamLimits},
    part::PartBodyReader,
    selector::{SelectorAction, SelectorEngine},
};

/// High-level multipart stream abstraction.
#[derive(Debug)]
pub struct Multipart<S> {
    inner: MultipartStream<S>,
    selector: SelectorEngine,
    limits: Limits,
    file_count: usize,
    field_count: usize,
}

impl<S> Multipart<S> {
    /// Creates a multipart stream from an already extracted boundary and a chunk source.
    pub fn new(boundary: impl Into<String>, stream: S) -> Result<Self, ParseError> {
        Ok(Self {
            inner: MultipartStream::new(boundary, stream)?,
            selector: SelectorEngine::new(Selector::any(), UnknownFieldPolicy::Ignore),
            limits: Limits::default(),
            file_count: 0,
            field_count: 0,
        })
    }

    /// Creates a multipart stream with explicit selector configuration.
    pub fn with_config(
        boundary: impl Into<String>,
        stream: S,
        config: MulterConfig,
    ) -> Result<Self, MulterError> {
        config.validate()?;
        let stream_limits = StreamLimits {
            max_file_size: config.limits.max_file_size,
            max_field_size: config.limits.max_field_size,
            max_body_size: config.limits.max_body_size,
        };
        let selector = SelectorEngine::new(config.selector, config.unknown_field_policy);
        Ok(Self {
            inner: MultipartStream::with_limits(boundary, stream, stream_limits)?,
            selector,
            limits: config.limits,
            file_count: 0,
            field_count: 0,
        })
    }
}

impl<S> Multipart<S>
where
    S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
{
    /// Returns the next multipart part, if available.
    pub async fn next_part(&mut self) -> Result<Option<Part<'_>>, MulterError> {
        loop {
            if self.inner.is_reading_part_body() {
                self.inner.drain_current_part().await?;
            }

            let headers = poll_fn(|cx| self.inner.poll_next_part_headers(cx)).await?;
            let Some(headers) = headers else {
                #[cfg(feature = "tracing")]
                tracing::debug!("multipart: reached end of stream");
                return Ok(None);
            };

            if headers.file_name.is_none() {
                self.field_count += 1;
                if let Some(max_fields) = self.limits.max_fields {
                    if self.field_count > max_fields {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(
                            max_fields = max_fields,
                            seen_fields = self.field_count,
                            "multipart: text field limit exceeded"
                        );
                        return Err(MulterError::FieldsLimitExceeded { max_fields });
                    }
                }

                #[cfg(feature = "tracing")]
                tracing::debug!(
                    field_name = headers.field_name.as_str(),
                    "multipart: yielding text part"
                );
                return Ok(Some(Part::new(headers, &mut self.inner)));
            }

            match self.selector.evaluate_file_field(&headers.field_name) {
                Ok(SelectorAction::Accept) => {
                    if let Some(patterns) = self.selector.field_allowed_mime_types(&headers.field_name) {
                        if !patterns.is_empty() && !mime_matches_any(&headers.content_type, patterns) {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(
                                field_name = headers.field_name.as_str(),
                                mime = headers.content_type.essence_str(),
                                "multipart: rejected by per-field MIME allowlist"
                            );
                            return Err(MulterError::MimeTypeNotAllowed {
                                field: headers.field_name.clone(),
                                mime: headers.content_type.essence_str().to_owned(),
                            });
                        }
                    }

                    if !self.limits.is_mime_allowed(&headers.content_type) {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(
                            field_name = headers.field_name.as_str(),
                            mime = headers.content_type.essence_str(),
                            "multipart: rejected by global MIME allowlist"
                        );
                        return Err(MulterError::MimeTypeNotAllowed {
                            field: headers.field_name.clone(),
                            mime: headers.content_type.essence_str().to_owned(),
                        });
                    }

                    self.file_count += 1;
                    if let Some(max_files) = self.limits.max_files {
                        if self.file_count > max_files {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(
                                max_files = max_files,
                                seen_files = self.file_count,
                                "multipart: file count limit exceeded"
                            );
                            return Err(MulterError::FilesLimitExceeded { max_files });
                        }
                    }

                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        field_name = headers.field_name.as_str(),
                        file_name = headers.file_name.as_deref().unwrap_or("<none>"),
                        mime = headers.content_type.essence_str(),
                        "multipart: yielding file part"
                    );
                    return Ok(Some(Part::new(headers, &mut self.inner)));
                }
                Ok(SelectorAction::Ignore) => {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        field_name = headers.field_name.as_str(),
                        "multipart: ignoring unmatched file field"
                    );
                    self.inner.drain_current_part().await?;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

impl<S> PartBodyReader for MultipartStream<S>
where
    S: Stream<Item = Result<Bytes, MulterError>> + Unpin,
{
    fn poll_next_chunk(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<Bytes>, MulterError>> {
        self.poll_next_part_chunk(cx)
    }
}

fn mime_matches_any(mime: &mime::Mime, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| mime_matches_pattern(mime, pattern))
}

fn mime_matches_pattern(mime: &mime::Mime, pattern: &str) -> bool {
    if let Some((kind, subtype)) = pattern.split_once('/') {
        if subtype == "*" {
            return mime.type_().as_str().eq_ignore_ascii_case(kind);
        }
    }

    mime.essence_str().eq_ignore_ascii_case(pattern)
}

