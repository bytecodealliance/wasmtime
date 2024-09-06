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

// Explicit methods with `#[allow]` to clearly indicate that truncation is
// desired when used.
#[allow(clippy::cast_possible_truncation)]
fn truncate_i32_to_i16(a: i32) -> i16 {
    a as i16
}

#[allow(clippy::cast_possible_truncation)]
fn truncate_i32_to_i8(a: i32) -> i8 {
    a as i8
}
