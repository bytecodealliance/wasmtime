use capstone;
use thiserror::Error;
use wasmparser::BinaryReaderError;

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
        let BinaryReaderError { message, offset } = e;
        Error::Input(format!("At wasm offset {}: {}", offset, message))
    }
}

impl From<capstone::Error> for Error {
    fn from(e: capstone::Error) -> Self {
        Error::Disassembler(e.to_string())
    }
}
