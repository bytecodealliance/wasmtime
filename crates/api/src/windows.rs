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

use crate::Instance;

/// Extensions for the [`Instance`] type only available on Windows.
pub trait InstanceExt {
    /// Configures a custom signal handler to execute.
    ///
    /// TODO: needs more documentation.
    unsafe fn set_signal_handler<H>(&self, handler: H)
    where
        H: 'static + Fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool;
}

impl InstanceExt for Instance {
    unsafe fn set_signal_handler<H>(&self, handler: H)
    where
        H: 'static + Fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool,
    {
        self.instance_handle.clone().set_signal_handler(handler);
    }
}
