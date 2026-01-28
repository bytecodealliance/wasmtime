//! Low-level allocation and OOM-handling utilities.

mod arc;
mod boxed;
mod try_new;
mod vec;

pub use boxed::{
    BoxedSliceFromFallibleIterError, BoxedSliceFromIterWithLenError,
    new_boxed_slice_from_fallible_iter, new_boxed_slice_from_iter,
    new_boxed_slice_from_iter_with_len,
};
pub use try_new::{TryNew, try_new};
pub use vec::Vec;

use crate::error::OutOfMemory;
use core::{alloc::Layout, ptr::NonNull};

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

/// An extension trait for ignoring `OutOfMemory` errors.
///
/// Use this to unwrap a `Result<T, OutOfMemory>` into its inner `T` or
/// otherwise panic, leveraging the type system to be sure that you aren't ever
/// accidentally unwrapping non-`OutOfMemory` errors.
pub trait PanicOnOom {
    /// The non-`OutOfMemory` result of calling `panic_on_oom`.
    type Result;

    /// Panic on `OutOfMemory` errors, returning the non-`OutOfMemory` result.
    fn panic_on_oom(self) -> Self::Result;
}

impl<T> PanicOnOom for Result<T, OutOfMemory> {
    type Result = T;

    #[track_caller]
    fn panic_on_oom(self) -> Self::Result {
        match self {
            Ok(x) => x,
            Err(oom) => panic!("unhandled out-of-memory error: {oom}"),
        }
    }
}
