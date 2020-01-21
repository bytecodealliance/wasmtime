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
        Error::Input(format!("At wasm offset {}: {}", e.offset(), e.message()))
    }
}

impl From<wasm_reader::Error> for Error {
    fn from(e: wasm_reader::Error) -> Self {
        match e {
            wasm_reader::Error::Error { error, offset } => {
                if let Some(o) = offset {
                    Error::Input(format!("At wasm offset {}: {}", o, error))
                } else {
                    Error::Input(format!("At wasm offset ???: {}", error))
                }
            }
            wasm_reader::Error::Eof => Error::Input(format!("At wasm offset ???: eof")),
        }
    }
}

impl From<capstone::Error> for Error {
    fn from(e: capstone::Error) -> Self {
        Error::Disassembler(e.to_string())
    }
}
