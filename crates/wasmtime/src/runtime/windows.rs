//! windows-specific extension for the `wasmtime` crate.
//!
//! This module is only available on Windows targets.
//! It is not available on Linux or macOS, for example. Note that the import path for
//! this module is `wasmtime::windows::...`, which is intended to emphasize that it
//! is platform-specific.
//!
//! The traits contained in this module are intended to extend various types
//! throughout the `wasmtime` crate with extra functionality that's only
//! available on Windows.

use crate::prelude::*;
use crate::{AsContextMut, Store};
use windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS;

/// Extensions for the [`Store`] type only available on Windows.
pub trait StoreExt {
    /// Configures a custom signal handler to execute.
    ///
    /// TODO: needs more documentation.
    #[cfg(feature = "signals-based-traps")]
    unsafe fn set_signal_handler<H>(&mut self, handler: H)
    where
        H: 'static + Fn(*mut EXCEPTION_POINTERS) -> bool + Send + Sync;
}

impl<T> StoreExt for Store<T> {
    #[cfg(feature = "signals-based-traps")]
    unsafe fn set_signal_handler<H>(&mut self, handler: H)
    where
        H: 'static + Fn(*mut EXCEPTION_POINTERS) -> bool + Send + Sync,
    {
        self.as_context_mut()
            .0
            .set_signal_handler(Some(Box::new(handler)));
    }
}
