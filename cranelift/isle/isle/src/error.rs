//! Error types.

use miette::{Diagnostic, SourceCode, SourceSpan};
use std::sync::Arc;

/// Either `Ok(T)` or `Err(isle::Error)`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by ISLE.
#[derive(thiserror::Error, Diagnostic, Clone, Debug)]
pub enum Error {
    /// An I/O error.
    #[error("{context}")]
    IoError {
        /// The underlying I/O error.
        #[source]
        error: Arc<std::io::Error>,
        /// The context explaining what caused the I/O error.
        context: String,
    },

    /// The input ISLE source has a parse error.
    #[error("parse error: {msg}")]
    #[diagnostic()]
    ParseError {
        /// The error message.
        msg: String,

        /// The input ISLE source.
        #[source_code]
        src: Source,

        /// The location of the parse error.
        #[label("{msg}")]
        span: SourceSpan,
    },

    /// The input ISLE source has a type error.
    #[error("type error: {msg}")]
    #[diagnostic()]
    TypeError {
        /// The error message.
        msg: String,

        /// The input ISLE source.
        #[source_code]
        src: Source,

        /// The location of the type error.
        #[label("{msg}")]
        span: SourceSpan,
    },

    /// Multiple errors.
    #[error("Found {} errors:\n\n{}",
            self.unwrap_errors().len(),
            DisplayErrors(self.unwrap_errors()))]
    #[diagnostic()]
    Errors(#[related] Vec<Error>),
}

impl Error {
    /// Create a `isle::Error` from the given I/O error and context.
    pub fn from_io(error: std::io::Error, context: impl Into<String>) -> Self {
        Error::IoError {
            error: Arc::new(error),
            context: context.into(),
        }
    }
}

impl From<Vec<Error>> for Error {
    fn from(es: Vec<Error>) -> Self {
        Error::Errors(es)
    }
}

impl Error {
    fn unwrap_errors(&self) -> &[Error] {
        match self {
            Error::Errors(e) => e,
            _ => panic!("`isle::Error::unwrap_errors` on non-`isle::Error::Errors`"),
        }
    }
}

struct DisplayErrors<'a>(&'a [Error]);
impl std::fmt::Display for DisplayErrors<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for e in self.0 {
            writeln!(f, "{}", e)?;
        }
        Ok(())
    }
}

/// A source file and its contents.
#[derive(Clone)]
pub struct Source {
    name: Arc<str>,
    text: Arc<str>,
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("name", &self.name)
            .field("source", &"<redacted>");
        Ok(())
    }
}

impl Source {
    pub(crate) fn new(name: Arc<str>, text: Arc<str>) -> Self {
        Self { name, text }
    }

    /// Get this source's file name.
    pub fn name(&self) -> &Arc<str> {
        &self.name
    }

    /// Get this source's text contents.
    pub fn text(&self) -> &Arc<str> {
        &self.name
    }
}

impl SourceCode for Source {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> std::result::Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        let contents = self
            .text
            .read_span(span, context_lines_before, context_lines_after)?;
        Ok(Box::new(miette::MietteSpanContents::new_named(
            self.name.to_string(),
            contents.data(),
            contents.span().clone(),
            contents.line(),
            contents.column(),
            contents.line_count(),
        )))
    }
}
