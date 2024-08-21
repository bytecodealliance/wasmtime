//! Implementation of garbage collection and GC types in Wasmtime.

mod arrayref;
mod data;
mod drc;
mod externref;
mod free_list;
mod structref;

pub use arrayref::*;
pub use data::*;
pub use drc::*;
pub use externref::*;
pub use structref::*;

use crate::runtime::vm::GcRuntime;

/// Get the default GC runtime.
pub fn default_gc_runtime() -> impl GcRuntime {
    DrcCollector
}

/// The default GC heap capacity: 512KiB.
#[cfg(not(miri))]
const DEFAULT_GC_HEAP_CAPACITY: usize = 1 << 19;

/// The default GC heap capacity for miri: 64KiB.
#[cfg(miri)]
const DEFAULT_GC_HEAP_CAPACITY: usize = 1 << 16;
