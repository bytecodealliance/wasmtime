//! Error types.

use std::sync::Arc;

/// Either `Ok(T)` or `Err(isle::Error)`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by ISLE.
#[derive(Clone, Debug)]
pub enum Error {
    /// An I/O error.
    IoError {
        /// The underlying I/O error.
        error: Arc<std::io::Error>,
        /// The context explaining what caused the I/O error.
        context: String,
    },

    /// The input ISLE source has a parse error.
    ParseError {
        /// The error message.
        msg: String,

        /// The input ISLE source.
        src: Source,

        /// The location of the parse error.
        span: Span,
    },

    /// The input ISLE source has a type error.
    TypeError {
        /// The error message.
        msg: String,

        /// The input ISLE source.
        src: Source,

        /// The location of the type error.
        span: Span,
    },

    /// Multiple errors.
    Errors(Vec<Error>),
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

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError { error, .. } => Some(&*error as &dyn std::error::Error),
            _ => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::IoError { context, .. } => write!(f, "{}", context),
            Error::ParseError { msg, .. } => write!(f, "parse error: {}", msg),
            Error::TypeError { msg, .. } => write!(f, "type error: {}", msg),
            Error::Errors(_) => write!(
                f,
                "found {} errors:\n\n{}",
                self.unwrap_errors().len(),
                DisplayErrors(self.unwrap_errors())
            ),
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
    /// The name of this source file.
    pub name: Arc<str>,
    /// The text of this source file.
    #[allow(unused)] // Used only when miette is enabled.
    pub text: Arc<str>,
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

/// A span in a given source.
#[derive(Clone, Debug)]
pub struct Span {
    /// The byte offset of the start of the span.
    pub from: usize,
    /// The byte offset of the end of the span.
    pub to: usize,
}

impl Span {
    /// Create a new span that covers one character at the given offset.
    pub fn new_single(offset: usize) -> Span {
        Span {
            from: offset,
            to: offset + 1,
        }
    }
}
