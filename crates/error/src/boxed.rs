use super::{OutOfMemory, Result};
use alloc::boxed::Box;
use core::alloc::Layout;
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
        out_of_line_slow_path!(Err(OutOfMemory::new()))
    }
}

/// Create a `Box<T>`, or return an `OutOfMemory` error.
#[inline]
pub(crate) fn try_box<T>(value: T) -> Result<Box<T>, OutOfMemory> {
    let layout = alloc::alloc::Layout::new::<T>();

    if layout.size() == 0 {
        // Safety: `Box` explicitly allows construction from dangling pointers
        // (which are guaranteed non-null and aligned) for zero-sized types.
        return Ok(unsafe { Box::from_raw(core::ptr::dangling::<T>().cast_mut()) });
    }

    // Safety: layout size is non-zero.
    let ptr = unsafe { try_alloc(layout)? };

    let ptr = ptr.cast::<T>();

    // Safety: The allocation succeeded, and it has `T`'s layout, so the pointer
    // is valid for writing a `T`.
    unsafe {
        ptr.write(value);
    }

    // Safety: The pointer's memory block was allocated by the global allocator,
    // is valid for `T`, and is initialized.
    Ok(unsafe { Box::from_raw(ptr.as_ptr()) })
}
