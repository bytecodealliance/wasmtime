use crate::instance::Instance;
use std::fmt;
use std::sync::Arc;

/// A struct representing an aborted instruction execution, with a message
/// indicating the cause.
#[derive(Clone)]
pub struct Trap {
    inner: Arc<TrapInner>,
}

struct TrapInner {
    message: String,
    trace: Vec<FrameInfo>,
}

fn _assert_trap_is_sync_and_send(t: &Trap) -> (&dyn Sync, &dyn Send) {
    (t, t)
}

impl Trap {
    /// Creates a new `Trap` with `message`.
    /// # Example
    /// ```
    /// let trap = wasmtime::Trap::new("unexpected error");
    /// assert_eq!("unexpected error", trap.message());
    /// ```
    pub fn new<I: Into<String>>(message: I) -> Self {
        Trap {
            inner: Arc::new(TrapInner {
                message: message.into(),
                trace: Vec::new(),
            }),
        }
    }

    /// Returns a reference the `message` stored in `Trap`.
    pub fn message(&self) -> &str {
        &self.inner.message
    }

    pub fn trace(&self) -> &[FrameInfo] {
        &self.inner.trace
    }
}

impl fmt::Debug for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Trap")
            .field("message", &self.inner.message)
            .field("trace", &self.inner.trace)
            .finish()
    }
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.message.fmt(f)
    }
}

impl std::error::Error for Trap {}

#[derive(Debug)]
pub struct FrameInfo;

impl FrameInfo {
    pub fn instance(&self) -> *const Instance {
        unimplemented!("FrameInfo::instance");
    }

    pub fn func_index() -> usize {
        unimplemented!("FrameInfo::func_index");
    }

    pub fn func_offset() -> usize {
        unimplemented!("FrameInfo::func_offset");
    }

    pub fn module_offset() -> usize {
        unimplemented!("FrameInfo::module_offset");
    }
}
