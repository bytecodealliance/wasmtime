//! Implementation of garbage collection and GC types in Wasmtime.

mod arrayref;
mod data;
mod externref;
#[cfg(feature = "gc-drc")]
mod free_list;
mod structref;

pub use arrayref::*;
pub use data::*;
pub use externref::*;
pub use structref::*;

#[cfg(feature = "gc-drc")]
mod drc;
#[cfg(feature = "gc-drc")]
pub use drc::*;

#[cfg(feature = "gc-null")]
mod null;
#[cfg(feature = "gc-null")]
pub use null::*;

/// The default GC heap capacity.
//
// Note that this is a bit smaller for miri to avoid overheads.
#[cfg(any(feature = "gc-drc", feature = "gc-null"))]
const DEFAULT_GC_HEAP_CAPACITY: usize = if cfg!(miri) { 1 << 16 } else { 1 << 19 };

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
