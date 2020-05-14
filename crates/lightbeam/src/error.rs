use std::fmt::Display;
use thiserror::Error;
use wasmparser::BinaryReaderError;

pub fn error_nopanic(inner: impl Into<String>) -> Error {
    Error::Microwasm(inner.into())
}

// For debugging, we have the option to panic when we hit an error so we can see the backtrace,
// as well as inspect state in `rr` or `gdb`.
// #[cfg(debug_assertions)]
#[allow(unreachable_code)]
pub fn error(inner: impl Into<String> + Display) -> Error {
    panic!(
        "`panic_on_error` feature enabled in `lightbeam`, this should be used for debugging \
        ONLY: {}",
        inner,
    );
    error_nopanic(inner)
}

// #[cfg(not(debug_assertions))]
// pub fn error(inner: impl Into<String> + Display) -> Error {
//     error_nopanic(inner)
// }

#[derive(Error, PartialEq, Eq, Clone, Debug)]
pub enum Error {
    #[error("Disassembler error: {0}")]
    Disassembler(String),

    #[error("Assembler error: {0}")]
    Assembler(String),

    #[error("Input error: {0}")]
    Input(String),

    #[error("Microwasm error: {0}")]
    Microwasm(String),
}

impl From<BinaryReaderError> for Error {
    fn from(e: BinaryReaderError) -> Self {
        Error::Input(format!("At wasm offset {}: {}", e.offset(), e.message()))
    }
}

impl From<capstone::Error> for Error {
    fn from(e: capstone::Error) -> Self {
        Error::Disassembler(e.to_string())
    }
}
