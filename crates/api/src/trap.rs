use thiserror::Error;

/// A struct representing an aborted instruction execution, with a message
/// indicating the cause.
#[derive(Error, Debug)]
#[error("Wasm trap: {message}")]
pub struct Trap {
    message: String,
}

impl Trap {
    /// Creates a new `Trap` with `message`.
    /// # Example
    /// ```
    /// let trap = wasmtime::Trap::new("unexpected error");
    /// assert_eq!("unexpected error", trap.message());
    /// ```
    pub fn new<I: Into<String>>(message: I) -> Trap {
        Self {
            message: message.into(),
        }
    }

    /// Create a `Trap` without defining a message for the trap. Mostly useful
    /// for prototypes and tests.
    pub fn fake() -> Trap {
        Self::new("TODO trap")
    }

    /// Returns a reference the `message` stored in `Trap`.
    pub fn message(&self) -> &str {
        &self.message
    }
}
