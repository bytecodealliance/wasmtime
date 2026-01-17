//! Fallible, OOM-handling collections.

use crate::error::OutOfMemory;
use core::{alloc::Layout, ptr::NonNull};

/// Internal macro to mark a block as a slow path, pulling it out into its own
/// cold function that is never inlined.
///
/// This should be applied to the whole consequent/alternative block for a
/// conditional, never to a single expression within a larger block.
///
/// # Example
///
/// ```ignore
/// fn hot_function(x: u32) -> Result<()> {
///     if very_rare_condition(x) {
///         return out_of_line_slow_path! {
///             // Handle the rare case...
///             //
///             // This pulls the handling of the rare condition out into
///             // its own, separate function, which keeps the generated code
///             // tight, handling only the common cases inline.
///             Ok(())
///         };
///     }
///
///     // Handle the common case inline...
///     Ok(())
/// }
/// ```
macro_rules! out_of_line_slow_path {
    ( $e:expr ) => {{
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn out_of_line_slow_path<T>(f: impl FnOnce() -> T) -> T {
            f()
        }

        out_of_line_slow_path(|| $e)
    }};
}

mod arc;
mod boxed;
pub use arc::OomArc;
pub use boxed::OomBox;

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
        out_of_line_slow_path! {
            Err(OutOfMemory::new(layout.size()))
        }
    }
}
