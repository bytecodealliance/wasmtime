//! Low-level allocation and OOM-handling utilities.

mod arc;
mod boxed;
mod try_new;

pub use boxed::{BoxedSliceFromIterError, new_boxed_slice_from_iter};
pub use try_new::{TryNew, try_new};

use core::{alloc::Layout, ptr::NonNull};
use wasmtime_error::OutOfMemory;

/// Try to allocate a block of memory that fits the given layout, or return an
/// `OutOfMemory` error.
///
/// # Safety
///
/// Same as `alloc::alloc::alloc`: layout must have non-zero size.
#[inline]
unsafe fn try_alloc(layout: Layout) -> Result<NonNull<u8>, OutOfMemory> {
    // Safety: same as our safety conditions.
    debug_assert!(layout.size() > 0);
    let ptr = unsafe { std_alloc::alloc::alloc(layout) };

    if let Some(ptr) = NonNull::new(ptr) {
        Ok(ptr)
    } else {
        Err(OutOfMemory::new(layout.size()))
    }
}
