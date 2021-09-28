//! Error types.

use crate::lexer::Pos;
use std::fmt;

/// Errors produced by ISLE.
#[derive(Clone, Debug)]
pub enum Error {
    /// The input ISLE source has an error.
    CompileError {
        /// The error message.
        msg: String,
        /// The ISLE source filename where the error occurs.
        filename: String,
        /// The position within the file that the error occurs at.
        pos: Pos,
    },
    /// An error from elsewhere in the system.
    SystemError {
        /// The error message.
        msg: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::CompileError {
                ref msg,
                ref filename,
                pos,
            } => {
                write!(f, "{}:{}:{}: error: {}", filename, pos.line, pos.col, msg)
            }
            &Error::SystemError { ref msg } => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for Error {}

impl std::convert::From<std::fmt::Error> for Error {
    fn from(e: std::fmt::Error) -> Error {
        Error::SystemError {
            msg: format!("{}", e),
        }
    }
}
impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::SystemError {
            msg: format!("{}", e),
        }
    }
}
