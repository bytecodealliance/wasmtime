#[derive(Fail, Debug)]
#[fail(display = "Wasm trap")]
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
