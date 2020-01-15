use crate::instance::Instance;
use backtrace::Backtrace;
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
    wasm_trace: Vec<FrameInfo>,
    native_trace: Backtrace,
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
        Trap::new_with_trace(message.into(), Backtrace::new_unresolved())
    }

    pub(crate) fn from_jit(jit: wasmtime_runtime::Trap) -> Self {
        Trap::new_with_trace(jit.to_string(), jit.backtrace)
    }

    fn new_with_trace(message: String, native_trace: Backtrace) -> Self {
        let mut wasm_trace = Vec::new();
        for frame in native_trace.frames() {
            let pc = frame.ip() as usize;
            if let Some(info) = wasmtime_runtime::jit_function_registry::find(pc) {
                wasm_trace.push(FrameInfo {
                    func_index: info.func_index as u32,
                    module_name: info.module_id.clone(),
                })
            }
        }
        Trap {
            inner: Arc::new(TrapInner {
                message,
                wasm_trace,
                native_trace,
            }),
        }
    }

    /// Returns a reference the `message` stored in `Trap`.
    pub fn message(&self) -> &str {
        &self.inner.message
    }

    pub fn trace(&self) -> &[FrameInfo] {
        &self.inner.wasm_trace
    }
}

impl fmt::Debug for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Trap")
            .field("message", &self.inner.message)
            .field("wasm_trace", &self.inner.wasm_trace)
            .field("native_trace", &self.inner.native_trace)
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
}
