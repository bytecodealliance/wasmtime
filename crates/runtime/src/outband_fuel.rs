//! Support for async fuel metering.

#![allow(missing_docs)]

use std::sync::Once;
use std::sync::{Arc, Mutex};

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        mod macos;
        use macos as sys;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        use linux as sys;
    } else {
        // TODO: I need to make it compile on other platforms.
    }
}

struct SharedInner(Option<sys::CheckHandle>);

/// This struct is owned by a store.
pub struct OutbandFuelSupport {
    inner: Arc<Mutex<SharedInner>>,
}

impl OutbandFuelSupport {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SharedInner(None))),
        }
    }

    pub fn new_checker(&self) -> OutbandFuelCheckHandle {
        let inner = self.inner.clone();
        OutbandFuelCheckHandle { inner }
    }

    pub fn enter_wasm(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.0 = Some(sys::CheckHandle::from_current_thread());
    }

    pub fn leave_wasm(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.0 = None;
    }
}

/// A handle that allows checking for the out-of-fuel condition from another thread.
pub struct OutbandFuelCheckHandle {
    inner: Arc<Mutex<SharedInner>>,
}

impl OutbandFuelCheckHandle {
    // TODO: Consider what happens in case different threads try to check at the same time.
    pub fn check(&self) {
        if let Ok(ref inner) = self.inner.try_lock() {
            inner.0.as_ref().map(|handle| handle.check());
        }
    }
}

/// Globally-set callback to determine whether a program counter points at wasm
/// generated code. Note, that trampolines are specifically excluded.
///
/// This is initialized during `init_traps` below. The definition lives within
/// `wasmtime` currently.
pub(crate) static mut IS_WASM_PC: fn(usize) -> bool = |_| false;

/// This function is required to be called before a call to wasm code that supports out-of-band
/// fuel metering.
pub fn init_outband_fuel(is_wasm_pc: fn(usize) -> bool) {
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        IS_WASM_PC = is_wasm_pc;
        sys::platform_init();
    });
}
