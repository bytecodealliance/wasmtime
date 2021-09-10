//! Error types.

use crate::lexer::Pos;
use std::fmt;

#[derive(Clone, Debug)]
pub enum Error {
    CompileError {
        msg: String,
        filename: String,
        pos: Pos,
    },
    SystemError {
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
                write!(f, "{}:{}: {}", filename, pos.line, msg)
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
