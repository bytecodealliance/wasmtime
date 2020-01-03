use crate::instance::Instance;
use std::fmt;
use std::sync::Arc;
use wasmtime_runtime::{get_backtrace, Backtrace, BacktraceFrame};

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
        Self::new_with_trace(message, get_backtrace())
    }

    pub(crate) fn new_with_trace<I: Into<String>>(message: I, backtrace: Backtrace) -> Self {
        let mut trace = Vec::with_capacity(backtrace.len());
        for i in 0..backtrace.len() {
            // Don't include frames without backtrace info.
            if let Some(info) = FrameInfo::try_from(backtrace[i]) {
                trace.push(info);
            }
        }
        Trap {
            inner: Arc::new(TrapInner {
                message: message.into(),
                trace,
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
pub struct FrameInfo {
    module_name: Option<String>,
    func_index: u32,
}

impl FrameInfo {
    pub fn instance(&self) -> *const Instance {
        unimplemented!("FrameInfo::instance");
    }

    pub fn func_index(&self) -> u32 {
        self.func_index
    }

    pub fn func_offset(&self) -> usize {
        unimplemented!("FrameInfo::func_offset");
    }

    pub fn module_offset(&self) -> usize {
        unimplemented!("FrameInfo::module_offset");
    }

    pub fn module_name(&self) -> Option<&str> {
        self.module_name.as_deref()
    }

    pub(crate) fn try_from(backtrace: BacktraceFrame) -> Option<FrameInfo> {
        if let Some(tag) = backtrace.tag() {
            let func_index = tag.func_index as u32;
            let module_name = tag.module_id.clone();
            Some(FrameInfo {
                func_index,
                module_name,
            })
        } else {
            None
        }
    }
}
