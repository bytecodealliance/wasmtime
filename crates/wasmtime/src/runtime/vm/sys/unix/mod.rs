//! Implementation of Wasmtime's system primitives for Unix-like operating
//! systems.
//!
//! This module handles Linux and macOS for example.

#[cfg(has_virtual_memory)]
pub mod mmap;
pub mod traphandlers;
#[cfg(has_host_compiler_backend)]
pub mod unwind;
#[cfg(has_virtual_memory)]
pub mod vm;

#[cfg(all(has_native_signals, target_vendor = "apple"))]
pub mod machports;
#[cfg(has_native_signals)]
pub mod signals;

#[cfg(all(target_os = "linux", target_pointer_width = "64", feature = "std"))]
mod pagemap;
#[cfg(not(all(target_os = "linux", target_pointer_width = "64", feature = "std")))]
use crate::vm::pagemap_disabled as pagemap;

#[path = "../std_tls.rs"]
mod std_tls;
pub use std_tls::*;
