//! Custom platform support in Wasmtime.
//!
//! This module contains an implementation of defining Wasmtime's platform
//! support in terms of a minimal C API. This API can be found in the `capi`
//! module and all other functionality here is implemented in terms of that
//! module.
//!
//! For more information about this see `./examples/min-platform` as well as
//! `./docs/examples-minimal.md`.

#![warn(dead_code, unused_imports)]

#[cfg(feature = "signals-based-traps")]
use crate::prelude::*;

pub mod capi;
#[cfg(feature = "signals-based-traps")]
pub mod mmap;
pub mod traphandlers;
pub mod unwind;
#[cfg(feature = "signals-based-traps")]
pub mod vm;

#[cfg(feature = "signals-based-traps")]
fn cvt(rc: i32) -> Result<()> {
    match rc {
        0 => Ok(()),
        code => bail!("os error {code}"),
    }
}

#[inline]
pub fn tls_get() -> *mut u8 {
    unsafe { capi::wasmtime_tls_get() }
}

#[inline]
pub fn tls_set(ptr: *mut u8) {
    unsafe { capi::wasmtime_tls_set(ptr) }
}
