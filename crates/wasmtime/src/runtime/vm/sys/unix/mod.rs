//! Implementation of Wasmtime's system primitives for Unix-like operating
//! systems.
//!
//! This module handles Linux and macOS for example.

use core::cell::Cell;

#[cfg(feature = "signals-based-traps")]
pub mod mmap;
pub mod traphandlers;
pub mod unwind;
#[cfg(feature = "signals-based-traps")]
pub mod vm;

#[cfg(all(feature = "signals-based-traps", target_vendor = "apple"))]
pub mod machports;
#[cfg(feature = "signals-based-traps")]
pub mod signals;

std::thread_local!(static TLS: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) });

#[inline]
pub fn tls_get() -> *mut u8 {
    TLS.with(|p| p.get())
}

#[inline]
pub fn tls_set(ptr: *mut u8) {
    TLS.with(|p| p.set(ptr));
}
