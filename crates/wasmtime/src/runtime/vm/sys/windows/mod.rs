//! Implementation of Wasmtime's system primitives for Windows.

#[cfg(has_virtual_memory)]
pub mod mmap;
pub mod traphandlers;
#[cfg(has_native_signals)]
mod vectored_exceptions;
pub mod vm;

#[cfg(all(target_pointer_width = "64", has_host_compiler_backend))]
pub mod unwind64;
#[cfg(all(target_pointer_width = "64", has_host_compiler_backend))]
pub use unwind64 as unwind;

#[cfg(all(not(target_pointer_width = "64"), has_host_compiler_backend))]
compile_error!("don't know how to unwind non-64 bit platforms");

#[path = "../std_tls.rs"]
mod std_tls;
pub use std_tls::*;
