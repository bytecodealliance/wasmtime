//! Functions for converting a reference into a singleton slice.
//!
//! See also the [`ref_slice` crate](https://crates.io/crates/ref_slice).
//!
//! We define the functions here to avoid external dependencies, and to ensure that they are
//! inlined in this crate.
//!
//! Despite their using an unsafe block, these functions are completely safe.

use std::slice;

pub fn ref_slice<T>(s: &T) -> &[T] {
    unsafe { slice::from_raw_parts(s, 1) }
}

pub fn ref_slice_mut<T>(s: &mut T) -> &mut [T] {
    unsafe { slice::from_raw_parts_mut(s, 1) }
}
