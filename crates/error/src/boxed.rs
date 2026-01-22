use super::{OutOfMemory, Result};
use alloc::boxed::Box;
use core::alloc::Layout;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

/// Try to allocate a block of memory that fits the given layout, or return an
/// `OutOfMemory` error.
///
/// # Safety
///
/// Same as `alloc::alloc::alloc`: layout must have non-zero size.
#[inline]
pub(crate) unsafe fn try_alloc(layout: Layout) -> Result<NonNull<u8>, OutOfMemory> {
    // Safety: same as our safety conditions.
    debug_assert!(layout.size() > 0);
    let ptr = unsafe { alloc::alloc::alloc(layout) };

    if let Some(ptr) = NonNull::new(ptr) {
        Ok(ptr)
    } else {
        Err(OutOfMemory::new(layout.size()))
    }
}

/// Create a `Box<T>`, or return an `OutOfMemory` error.
#[inline]
pub(crate) fn try_new_uninit_box<T>() -> Result<Box<MaybeUninit<T>>, OutOfMemory> {
    let layout = alloc::alloc::Layout::new::<MaybeUninit<T>>();

    if layout.size() == 0 {
        // NB: no actual allocation takes place when boxing zero-sized types.
        return Ok(Box::new(MaybeUninit::uninit()));
    }

    // Safety: layout size is non-zero.
    let ptr = unsafe { try_alloc(layout)? };

    let ptr = ptr.cast::<MaybeUninit<T>>();

    // Safety: The pointer's memory block was allocated by the global allocator.
    Ok(unsafe { Box::from_raw(ptr.as_ptr()) })
}
