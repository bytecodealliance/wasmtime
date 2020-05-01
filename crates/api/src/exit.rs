use std::fmt;
use std::num::NonZeroI32;

/// A struct representing an explicit program exit, with a code
/// indicating the status.
#[derive(Clone, Debug)]
pub struct Exit {
    status: NonZeroI32,
}

impl Exit {
    /// Creates a new `Exit` with `status`. The status value must be in the range [1..126].
    /// # Example
    /// ```
    /// let exit = wasmtime::Exit::new(std::num::NonZeroI32::new(1).unwrap());
    /// assert_eq!(1, exit.status().get());
    /// ```
    pub fn new(status: NonZeroI32) -> Self {
        assert!(status.get() > 0 && status.get() < 126);
        Self { status }
    }

    /// Returns a reference the `status` stored in `Exit`.
    pub fn status(&self) -> NonZeroI32 {
        self.status
    }
}

impl fmt::Display for Exit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Error codes used by WASI libc.
        match self.status.get() {
            70 => write!(f, "internal software error"),
            71 => write!(f, "system error"),
            _ => write!(f, "non-zero status {}", self.status),
        }
    }
}

impl std::error::Error for Exit {}
