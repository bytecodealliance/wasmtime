//! Implementation of Wasmtime's system primitives for Unix-like operating
//! systems.
//!
//! This module handles Linux and macOS for example.

use core::cell::Cell;

pub mod mmap;
pub mod unwind;
pub mod vm;

pub mod signals;

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        pub mod machports;

        pub mod macos_traphandlers;
        pub use macos_traphandlers as traphandlers;
    } else {
        pub use signals as traphandlers;
    }
}

std::thread_local!(static TLS: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) });

#[inline]
pub fn tls_get() -> *mut u8 {
    TLS.with(|p| p.get())
}

#[inline]
pub fn tls_set(ptr: *mut u8) {
    TLS.with(|p| p.set(ptr));
}
