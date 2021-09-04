//! Error types.

use crate::lexer::Pos;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Parse error")]
    ParseError(#[from] ParseError),
    #[error("Semantic error")]
    SemaError(#[from] SemaError),
    #[error("IO error")]
    IoError(#[from] std::io::Error),
    #[error("Formatting error")]
    FmtError(#[from] std::fmt::Error),
}

#[derive(Clone, Debug, Error)]
pub struct ParseError {
    pub msg: String,
    pub filename: String,
    pub pos: Pos,
}

#[derive(Clone, Debug, Error)]
pub struct SemaError {
    pub msg: String,
    pub filename: String,
    pub pos: Pos,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.filename, self.pos.line, self.pos.col, self.msg
        )
    }
}

impl std::fmt::Display for SemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.filename, self.pos.line, self.pos.col, self.msg
        )
    }
}
