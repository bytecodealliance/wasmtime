use std::sync::Arc;

/// TLS error
pub struct Error(Arc<String>);

impl Error {
    /// Creates a new error with the given message.
    pub fn msg<M>(message: M) -> Self
    where
        M: ToString,
    {
        Self(Arc::new(message.to_string()))
    }
}
impl Clone for Error {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        // Try to recover the original error:
        match err.downcast::<Error>() {
            Ok(e) => e,
            Err(io_err) => Self::msg(io_err),
        }
    }
}
impl From<Error> for std::io::Error {
    fn from(err: Error) -> Self {
        std::io::Error::other(err)
    }
}
