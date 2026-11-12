//! Implementation of garbage collection and GC types in Wasmtime.

mod arrayref;
mod exnref;
mod externref;
#[cfg(feature = "gc-drc")]
mod free_list;
mod structref;
#[cfg(any(feature = "gc-drc", feature = "gc-copying"))]
mod trace_infos;

pub use arrayref::*;
pub use exnref::*;
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

#[cfg(feature = "gc-copying")]
mod copying;
#[cfg(feature = "gc-copying")]
pub use copying::*;

/// Returns whether every byte in `bytes` is [`wasmtime_environ::POISON`].
#[cfg(any(feature = "gc-drc", feature = "gc-copying"))]
fn is_poisoned(bytes: &[u8]) -> bool {
    // Check `bytes` in large-ish chunks. This forces compiling down to `memcmp`
    // in both debug mode and with sanitizers enabled (fuzzing).
    const CHUNK: usize = 4096;
    static POISONED: [u8; CHUNK] = [wasmtime_environ::POISON; CHUNK];
    bytes
        .chunks(CHUNK)
        .all(|chunk| chunk == &POISONED[..chunk.len()])
}
