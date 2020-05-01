//! `Error` and `Result` types for this crate.

use std::io;
use thiserror::Error;

/// A result type containing `Ok(T)` or `Err(peepmatic_runtime::Error)`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that `peepmatic_runtime` may generate.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error {
    #[from]
    inner: Box<ErrorInner>,
}

#[derive(Debug, Error)]
enum ErrorInner {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Bincode(#[from] bincode::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        let e: ErrorInner = e.into();
        e.into()
    }
}

impl From<bincode::Error> for Error {
    fn from(e: bincode::Error) -> Error {
        let e: ErrorInner = e.into();
        e.into()
    }
}

impl From<ErrorInner> for Error {
    fn from(e: ErrorInner) -> Error {
        Box::new(e).into()
    }
}
