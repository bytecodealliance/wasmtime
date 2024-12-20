//! Implementation of Wasmtime's system primitives for Windows.

use std::cell::Cell;

pub mod mmap;
pub mod traphandlers;
pub mod vm;

#[cfg(target_pointer_width = "32")]
pub mod unwind32;
#[cfg(target_pointer_width = "32")]
pub use unwind32 as unwind;
#[cfg(target_pointer_width = "64")]
pub mod unwind64;
#[cfg(target_pointer_width = "64")]
pub use unwind64 as unwind;

std::thread_local!(static TLS: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) });

#[inline]
pub fn tls_get() -> *mut u8 {
    TLS.with(|p| p.get())
}

#[inline]
pub fn tls_set(ptr: *mut u8) {
    TLS.with(|p| p.set(ptr));
}
