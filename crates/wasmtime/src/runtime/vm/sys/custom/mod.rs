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

#[cfg(has_virtual_memory)]
use crate::prelude::*;

pub mod capi;
#[cfg(has_virtual_memory)]
pub mod mmap;
pub mod traphandlers;
#[cfg(has_host_compiler_backend)]
pub mod unwind;
#[cfg(has_virtual_memory)]
pub mod vm;

#[cfg(has_virtual_memory)]
fn cvt(rc: i32) -> Result<()> {
    match rc {
        0 => Ok(()),
        code => bail!("os error {code}"),
    }
}

#[inline]
pub fn tls_get() -> *mut u8 {
    unsafe { capi::wasmtime_tls_get(0) }
}

#[inline]
pub fn tls_set(ptr: *mut u8) {
    unsafe { capi::wasmtime_tls_set(0, ptr) }
}

#[inline]
#[cfg(feature = "component-model-async")]
pub fn component_async_tls_get() -> *mut u8 {
    unsafe { capi::wasmtime_tls_get(1) }
}

#[inline]
#[cfg(feature = "component-model-async")]
pub fn component_async_tls_set(ptr: *mut u8) {
    unsafe { capi::wasmtime_tls_set(1, ptr) }
}
