//! Error types.

use std::sync::Arc;

use crate::lexer::Pos;

/// Errors produced by ISLE.
#[derive(thiserror::Error, Clone, Debug)]
pub enum Error {
    /// An I/O error.
    #[error(transparent)]
    IoError(Arc<std::io::Error>),

    /// The input ISLE source has an error.
    #[error("{}:{}:{}: {}", .filename, .pos.line, .pos.col, .msg)]
    CompileError {
        /// The error message.
        msg: String,
        /// The ISLE source filename where the error occurs.
        filename: String,
        /// The position within the file that the error occurs at.
        pos: Pos,
    },
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(Arc::new(e))
    }
}
