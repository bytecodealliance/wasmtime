use thiserror::Error;

#[derive(Error, Debug)]
#[error("Wasm trap: {message}")]
pub struct Trap {
    message: String,
}

impl Trap {
    pub fn new(message: String) -> Trap {
        Trap { message }
    }

    pub fn fake() -> Trap {
        Trap::new("TODO trap".to_string())
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
