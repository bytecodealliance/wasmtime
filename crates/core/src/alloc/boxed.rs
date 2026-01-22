use super::{TryNew, try_alloc};
use core::{alloc::Layout, mem::MaybeUninit};
use std_alloc::boxed::Box;
use wasmtime_error::OutOfMemory;

/// Allocate an `Box<MaybeUninit<T>>` with uninitialized contents, returning
/// `Err(OutOfMemory)` on allocation failure.
///
/// You can initialize the resulting box's value via [`Box::write`].
#[inline]
fn new_uninit_box<T>() -> Result<Box<MaybeUninit<T>>, OutOfMemory> {
    let layout = Layout::new::<MaybeUninit<T>>();

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
