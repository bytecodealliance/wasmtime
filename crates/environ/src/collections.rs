//! Fallible, OOM-handling collections.

use crate::error::OutOfMemory;
use alloc::{boxed::Box, sync::Arc};
use core::{alloc::Layout, mem::MaybeUninit, ptr::NonNull};

/// Helper function to invoke `<T as TryNew>::try_new`.
///
/// # Example
///
/// ```
/// # use wasmtime_environ::prelude::*;
/// # fn _foo() -> Result<()> {
/// use wasmtime_environ::collections::try_new;
///
/// let boxed = try_new::<Box<u32>>(36)?;
/// assert_eq!(*boxed, 36);
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn try_new<T>(value: T::Value) -> Result<T, OutOfMemory>
where
    T: TryNew,
{
    TryNew::try_new(value)
}

/// Extension trait providing fallible allocation for types like `Arc<T>` and
/// `Box<T>.
pub trait TryNew {
    /// The inner `T` type that is getting wrapped into an `Arc<T>` or `Box<T>`.
    type Value;

    /// Allocate a new `Self`, returning `Err(OutOfMemory)` on allocation
    /// failure.
    fn try_new(value: Self::Value) -> Result<Self, OutOfMemory>
    where
        Self: Sized;
}

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
    let ptr = unsafe { alloc::alloc::alloc(layout) };

    if let Some(ptr) = NonNull::new(ptr) {
        Ok(ptr)
    } else {
        Err(OutOfMemory::new(layout.size()))
    }
}

/// Allocate an `Box<MaybeUninit<T>>` with uninitialized contents, returning
/// `Err(OutOfMemory)` on allocation failure.
///
/// You can initialize the resulting box's value via [`Box::write`].
#[inline]
fn new_uninit_box<T>() -> Result<Box<MaybeUninit<T>>, OutOfMemory> {
    let layout = alloc::alloc::Layout::new::<MaybeUninit<T>>();

    if layout.size() == 0 {
        // NB: no actual allocation takes place when boxing zero-sized
        // types.
        return Ok(Box::new(MaybeUninit::uninit()));
    }

    // Safety: layout size is non-zero.
    let ptr = unsafe { try_alloc(layout)? };

    let ptr = ptr.cast::<MaybeUninit<T>>();

    // Safety: The pointer's memory block was allocated by the global allocator.
    Ok(unsafe { Box::from_raw(ptr.as_ptr()) })
}

impl<T> TryNew for Box<T> {
    type Value = T;

    #[inline]
    fn try_new(value: T) -> Result<Self, OutOfMemory>
    where
        Self: Sized,
    {
        let boxed = new_uninit_box::<T>()?;
        Ok(Box::write(boxed, value))
    }
}

/// XXX: Stable Rust doesn't actually give us any method to build fallible
/// allocation for `Arc<T>`, so this is only actually fallible when using
/// nightly Rust and setting `RUSTFLAGS="--cfg arc_try_new"`.
impl<T> TryNew for Arc<T> {
    type Value = T;

    #[inline]
    fn try_new(value: T) -> Result<Self, OutOfMemory>
    where
        Self: Sized,
    {
        #[cfg(arc_try_new)]
        return Arc::try_new(value).map_err(|_| {
            // We don't have access to the exact size of the inner `Arc`
            // allocation, but (at least at one point) it was made up of a
            // strong ref count, a weak ref count, and the inner value.
            let bytes = core::mem::size_of::<(usize, usize, T)>();
            OutOfMemory::new(bytes)
        });

        #[cfg(not(arc_try_new))]
        return Ok(Arc::new(value));
    }
}
