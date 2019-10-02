use capstone;
use wasmparser::BinaryReaderError;

#[derive(Fail, PartialEq, Eq, Clone, Debug)]
pub enum Error {
    #[fail(display = "Disassembler error: {}", _0)]
    Disassembler(String),

    #[fail(display = "Assembler error: {}", _0)]
    Assembler(String),

    #[fail(display = "Input error: {}", _0)]
    Input(String),
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
