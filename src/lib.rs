#![warn(missing_docs)]
#![forbid(unsafe_code)]

//! Core crate surface for `rust-multer`.

use bytes::Bytes;
use futures::{Stream, StreamExt};
use tokio::io::AsyncRead;
use tokio_util::io::ReaderStream;

/// Fluent builder API.
pub mod builder;
/// Multipart parser configuration.
pub mod config;
/// Error types exposed by this crate.
pub mod error;
/// Field selection and matching models.
pub mod field;
/// Request and field limits.
pub mod limits;
/// High-level multipart stream type.
pub mod multipart;
/// Parsed multipart part API.
pub mod part;
/// Runtime selector engine.
pub mod selector;
/// Low-level parser components.
pub mod parser;
/// Storage engine traits and implementations.
pub mod storage;

#[cfg(feature = "actix")]
pub mod actix;
#[cfg(feature = "axum")]
pub mod axum;

pub use builder::MulterBuilder;
pub use config::{MulterConfig, SelectedField, Selector, UnknownFieldPolicy};
pub use error::{ConfigError, MulterError, ParseError, StorageError};
pub use field::{Field, FieldKind, FileField, TextField};
pub use limits::Limits;
pub use multipart::Multipart;
pub use part::Part;
pub use selector::{SelectorAction, SelectorEngine};
pub use storage::{
    BoxStream, DiskStorage, DiskStorageBuilder, FileMeta, FilenameStrategy, MemoryStorage,
    NoopStorage, StorageEngine, StoredFile,
};

/// `AsyncRead` adapter stream used by [`Multer::parse_stream`].
pub type AsyncReadStream<R> =
    futures::stream::Map<ReaderStream<R>, fn(Result<Bytes, std::io::Error>) -> Result<Bytes, MulterError>>;

/// Processed multipart output returned by [`Multer::parse_and_store`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedMultipart<O = StoredFile> {
    /// File parts persisted through the configured storage engine.
    pub stored_files: Vec<O>,
    /// Text field values collected from the stream.
    pub text_fields: Vec<(String, String)>,
}

impl<O> Default for ProcessedMultipart<O> {
    fn default() -> Self {
        Self {
            stored_files: Vec::new(),
            text_fields: Vec::new(),
        }
    }
}

/// Main `rust-multer` entry point.
#[derive(Debug)]
pub struct Multer<S = NoopStorage> {
    config: MulterConfig,
    storage: S,
}

impl<S> Multer<S> {
    /// Creates a new multer instance with the given storage backend.
    ///
    /// ```rust
    /// use rust_multer::{MemoryStorage, Multer};
    ///
    /// let multer = Multer::new(MemoryStorage::new());
    /// assert!(multer.config().limits.allowed_mime_types.is_empty());
    /// ```
    pub fn new(storage: S) -> Self {
        Self {
            config: MulterConfig::default(),
            storage,
        }
    }

    /// Creates a new multer instance with explicit validated configuration.
    pub fn with_config(storage: S, config: MulterConfig) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self { config, storage })
    }

    /// Returns an immutable reference to the active configuration.
    pub fn config(&self) -> &MulterConfig {
        &self.config
    }

    /// Returns an immutable reference to the configured storage backend.
    pub fn storage(&self) -> &S {
        &self.storage
    }
}

impl<S> Multer<S>
where
    S: StorageEngine,
{
    /// Stores a file part through the configured storage backend.
    pub async fn store(&self, mut part: Part<'_>) -> Result<S::Output, MulterError> {
        let field_name = part.field_name().to_owned();
        let file_name = part.file_name().map(ToOwned::to_owned);
        let content_type = part.content_type().to_string();
        let stream = part.stream()?;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            field_name = field_name.as_str(),
            file_name = file_name.as_deref().unwrap_or("<none>"),
            content_type = content_type.as_str(),
            "multer: dispatching part to storage engine"
        );

        self.storage
            .store(&field_name, file_name.as_deref(), &content_type, stream)
            .await
            .map_err(|err| MulterError::Storage(StorageError::new(err.to_string())))
    }

    /// Creates a configured multipart parser from a raw multipart boundary.
    pub fn multipart_from_boundary<T>(
        &self,
        boundary: impl Into<String>,
        stream: T,
    ) -> Result<Multipart<T>, MulterError>
    where
        T: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        Multipart::with_config(boundary, stream, self.config.clone())
    }

    /// Creates a configured multipart parser from an HTTP `Content-Type` value.
    pub fn multipart_from_content_type<T>(
        &self,
        content_type: &str,
        stream: T,
    ) -> Result<Multipart<T>, MulterError>
    where
        T: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        let boundary = parser::extract_multipart_boundary(content_type)?;
        self.multipart_from_boundary(boundary, stream)
    }

    /// Creates a configured multipart parser from any `AsyncRead` body stream.
    ///
    /// ```rust
    /// use rust_multer::{MemoryStorage, Multer};
    /// use tokio::io::AsyncWriteExt;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let multer = Multer::new(MemoryStorage::new());
    /// let body = b"--BOUND\r\nContent-Disposition: form-data; name=\"field\"\r\n\r\nvalue\r\n--BOUND--\r\n";
    /// let (mut writer, reader) = tokio::io::duplex(1024);
    /// writer.write_all(body).await.expect("write body");
    /// drop(writer);
    ///
    /// let mut multipart = multer.parse_stream(reader, "BOUND").await.expect("parse stream");
    /// let mut part = multipart.next_part().await.expect("next part").expect("part");
    /// assert_eq!(part.text().await.expect("text"), "value");
    /// # }
    /// ```
    pub async fn parse_stream<R>(
        &self,
        stream: R,
        boundary: impl Into<String>,
    ) -> Result<Multipart<AsyncReadStream<R>>, MulterError>
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        self.multipart_from_boundary(boundary, map_async_read_stream(stream))
    }

    /// Parses multipart input and stores all file parts using the active storage backend.
    ///
    /// ```rust
    /// use bytes::Bytes;
    /// use futures::stream;
    /// use rust_multer::{MemoryStorage, Multer, MulterError};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let multer = Multer::new(MemoryStorage::new());
    /// let body = concat!(
    ///     "--BOUND\r\n",
    ///     "Content-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n",
    ///     "\r\n",
    ///     "hello\r\n",
    ///     "--BOUND--\r\n"
    /// );
    ///
    /// let output = multer
    ///     .parse_and_store(
    ///         "BOUND",
    ///         stream::iter([Ok::<Bytes, MulterError>(Bytes::from_static(body.as_bytes()))]),
    ///     )
    ///     .await
    ///     .expect("parse and store");
    ///
    /// assert_eq!(output.stored_files.len(), 1);
    /// # }
    /// ```
    pub async fn parse_and_store<T>(
        &self,
        boundary: impl Into<String>,
        stream: T,
    ) -> Result<ProcessedMultipart<S::Output>, MulterError>
    where
        T: Stream<Item = Result<Bytes, MulterError>> + Unpin,
    {
        let mut multipart = self.multipart_from_boundary(boundary, stream)?;
        let mut out = ProcessedMultipart::default();

        while let Some(mut part) = multipart.next_part().await? {
            if part.file_name().is_some() {
                #[cfg(feature = "tracing")]
                tracing::trace!(field_name = part.field_name(), "multer: storing file part");
                let stored = self.store(part).await?;
                out.stored_files.push(stored);
            } else {
                let field_name = part.field_name().to_owned();
                let text = part.text().await?;
                #[cfg(feature = "tracing")]
                tracing::trace!(field_name = field_name.as_str(), "multer: captured text part");
                out.text_fields.push((field_name, text));
            }
        }

        Ok(out)
    }
}

fn map_async_read_stream<R>(stream: R) -> AsyncReadStream<R>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    ReaderStream::new(stream).map(async_read_item_to_multer)
}

fn async_read_item_to_multer(item: Result<Bytes, std::io::Error>) -> Result<Bytes, MulterError> {
    item.map_err(|err| ParseError::new(format!("async read stream error: {err}")).into())
}

impl Multer<NoopStorage> {
    /// Creates a fluent builder with permissive defaults.
    ///
    /// ```rust
    /// use rust_multer::{Multer, UnknownFieldPolicy};
    ///
    /// let multer = Multer::builder()
    ///     .any()
    ///     .max_files(5)
    ///     .on_unknown_field(UnknownFieldPolicy::Reject)
    ///     .build()
    ///     .expect("builder config");
    ///
    /// assert_eq!(multer.config().limits.max_files, Some(5));
    /// ```
    pub fn builder() -> MulterBuilder {
        MulterBuilder::default()
    }
}

