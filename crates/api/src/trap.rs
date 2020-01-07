use crate::instance::Instance;
use crate::r#ref::HostRef;
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;

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

#[derive(Debug)]
pub struct TrapInfo {
    message: String,
    trace: Vec<FrameInfo>,
}

impl TrapInfo {
    pub fn new<I: Into<String>>(message: I) -> Self {
        Self {
            message: message.into(),
            trace: vec![],
        }
    }

    /// Returns a reference the `message` stored in `Trap`.
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn origin(&self) -> Option<&FrameInfo> {
        self.trace.first()
    }

    pub fn trace(&self) -> &[FrameInfo] {
        &self.trace
    }
}

/// A struct to hold unsafe TrapInfo host reference, designed
/// to be Send-able. The only access for it provided via the
/// Trap::trap_info_unchecked() method.
struct UnsafeTrapInfo(HostRef<TrapInfo>);

impl UnsafeTrapInfo {
    fn trap_info(&self) -> HostRef<TrapInfo> {
        self.0.clone()
    }
}

unsafe impl Send for UnsafeTrapInfo {}

impl fmt::Debug for UnsafeTrapInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UnsafeTrapInfo")
    }
}

/// A struct representing an aborted instruction execution, with a message
/// indicating the cause.
#[derive(Error, Debug, Clone)]
#[error("Wasm trap: {message}")]
pub struct Trap {
    message: String,
    info: Arc<Mutex<UnsafeTrapInfo>>,
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
        Trap::from(HostRef::new(TrapInfo::new(message)))
    }

    /// Returns a reference the `message` stored in `Trap`.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns inner TrapInfo assotiated with the Trap.
    /// The method is unsafe: obtained TrapInfo is not thread safe.
    pub(crate) unsafe fn trap_info_unchecked(&self) -> HostRef<TrapInfo> {
        self.info.lock().unwrap().trap_info()
    }
}

impl From<HostRef<TrapInfo>> for Trap {
    fn from(trap_info: HostRef<TrapInfo>) -> Self {
        let message = trap_info.borrow().message().to_string();
        let info = Arc::new(Mutex::new(UnsafeTrapInfo(trap_info)));
        Trap { message, info }
    }
}
