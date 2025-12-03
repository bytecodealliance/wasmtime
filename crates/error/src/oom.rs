use super::OomOrDynError;
use core::fmt;

/// Out-of-memory error.
///
/// This error is the sentinel for allocation failure due to memory exhaustion.
///
/// Constructing an [`Error`][crate::Error] from an `OutOfMemory` does not
/// allocate.
///
/// Allocation failure inside any `Error` method that must allocate
/// (e.g. [`Error::context`][crate::Error::context]) will propagate an
/// `OutOfMemory` error.
#[derive(Clone, Copy, Default)]
pub struct OutOfMemory {
    _private: (),
}

impl fmt::Debug for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OutOfMemory")
    }
}

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("out of memory")
    }
}

impl core::error::Error for OutOfMemory {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}

impl OutOfMemory {
    /// Construct a new `OutOfMemory` error.
    ///
    /// This operation does not allocate.
    #[inline]
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

impl From<OutOfMemory> for OomOrDynError {
    fn from(OutOfMemory { _private: () }: OutOfMemory) -> Self {
        OomOrDynError {
            inner: OomOrDynError::OOM,
        }
    }
}
