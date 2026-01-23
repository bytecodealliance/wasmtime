use super::{TryNew, try_alloc};
use crate::error::OutOfMemory;
use core::{alloc::Layout, mem::MaybeUninit};
use std_alloc::boxed::Box;

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

fn new_uninit_boxed_slice<T>(len: usize) -> Result<Box<[MaybeUninit<T>]>, OutOfMemory> {
    let layout = Layout::array::<MaybeUninit<T>>(len)
        .map_err(|_| OutOfMemory::new(len.saturating_mul(core::mem::size_of::<T>())))?;

    if layout.size() == 0 {
        // NB: no actual allocation takes place when boxing zero-sized
        // types.
        return Ok(Box::new_uninit_slice(len));
    }

    // Safety: we just ensured that the new length is non-zero.
    debug_assert_ne!(layout.size(), 0);
    let ptr = unsafe { try_alloc(layout)? };

    let ptr = ptr.cast::<MaybeUninit<T>>().as_ptr();
    let ptr = core::ptr::slice_from_raw_parts_mut(ptr, len);

    // Safety: `ptr` points to a memory block that is valid for
    // `[MaybeUninit<T>; len]` and which was allocated by the global memory
    // allocator.
    let boxed = unsafe { Box::from_raw(ptr) };
    Ok(boxed)
}

/// An error returned by [`new_boxed_slice_from_iter`].
#[derive(Debug)]
pub enum BoxedSliceFromIterError {
    /// The iterator did not yield enough items to fill the boxed slice.
    TooFewItems,
    /// Failed to allocate space for the boxed slice.
    Oom(OutOfMemory),
}

impl From<OutOfMemory> for BoxedSliceFromIterError {
    fn from(oom: OutOfMemory) -> Self {
        Self::Oom(oom)
    }
}

impl core::fmt::Display for BoxedSliceFromIterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BoxedSliceFromIterError::TooFewItems => {
                f.write_str("The iterator did not yield enough items to fill the boxed slice")
            }
            BoxedSliceFromIterError::Oom(_) => {
                f.write_str("Failed to allocate space for the boxed slice")
            }
        }
    }
}

impl core::error::Error for BoxedSliceFromIterError {
    fn cause(&self) -> Option<&dyn core::error::Error> {
        match self {
            BoxedSliceFromIterError::TooFewItems => None,
            BoxedSliceFromIterError::Oom(oom) => Some(oom),
        }
    }
}

/// Create a `Box<[T]>` of length `len` from the given iterator's elements.
///
/// Returns an error on allocation failure, or if `iter` yields fewer than `len`
/// elements.
///
/// The iterator is dropped after `len` elements have been yielded, this
/// function does not check that the iterator yields exactly `len` elements.
pub fn new_boxed_slice_from_iter<T>(
    len: usize,
    iter: impl IntoIterator<Item = T>,
) -> Result<Box<[T]>, BoxedSliceFromIterError> {
    /// RAII guard to handle dropping the initialized elements of the boxed
    /// slice in the cases where we get too few items or the iterator panics.
    struct DropGuard<T> {
        boxed: Box<[MaybeUninit<T>]>,
        init_len: usize,
    }

    impl<T> Drop for DropGuard<T> {
        fn drop(&mut self) {
            debug_assert!(self.init_len <= self.boxed.len());

            if !core::mem::needs_drop::<T>() {
                return;
            }

            for elem in self.boxed.iter_mut().take(self.init_len) {
                // Safety: the elements in `self.boxed[..self.init_len]` are
                // valid and initialized and will not be used again.
                unsafe {
                    core::ptr::drop_in_place(elem.as_mut_ptr());
                }
            }
        }
    }

    let mut guard = DropGuard {
        boxed: new_uninit_boxed_slice(len)?,
        init_len: 0,
    };
    assert_eq!(len, guard.boxed.len());

    for (i, elem) in iter.into_iter().enumerate().take(len) {
        debug_assert!(i < len);
        debug_assert_eq!(guard.init_len, i);
        guard.boxed[i].write(elem);
        guard.init_len += 1;
    }

    debug_assert!(guard.init_len <= guard.boxed.len());

    if guard.init_len < guard.boxed.len() {
        return Err(BoxedSliceFromIterError::TooFewItems);
    }

    debug_assert_eq!(guard.init_len, guard.boxed.len());

    // Take the boxed slice out of the guard.
    let boxed = {
        guard.init_len = 0;
        let boxed = core::mem::take(&mut guard.boxed);
        core::mem::forget(guard);
        boxed
    };

    // Safety: we initialized all elements.
    let boxed = unsafe { boxed.assume_init() };

    Ok(boxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;
    use std_alloc::rc::Rc;

    struct SetFlagOnDrop(Rc<Cell<bool>>);

    impl Drop for SetFlagOnDrop {
        fn drop(&mut self) {
            let old_value = self.0.replace(true);
            assert_eq!(old_value, false);
        }
    }

    impl SetFlagOnDrop {
        fn new() -> (Rc<Cell<bool>>, Self) {
            let flag = Rc::new(Cell::new(false));
            (flag.clone(), SetFlagOnDrop(flag))
        }
    }

    #[test]
    fn try_new() {
        <Box<_> as TryNew>::try_new(4).unwrap();
    }

    #[test]
    fn new_boxed_slice_from_iter_smoke_test() {
        let slice = new_boxed_slice_from_iter(3, [42, 36, 1337]).unwrap();
        assert_eq!(&*slice, &[42, 36, 1337]);
    }

    #[test]
    fn new_boxed_slice_from_iter_with_too_few_elems() {
        let (a_dropped, a) = SetFlagOnDrop::new();
        let (b_dropped, b) = SetFlagOnDrop::new();
        let (c_dropped, c) = SetFlagOnDrop::new();

        match new_boxed_slice_from_iter(4, [a, b, c]) {
            Err(BoxedSliceFromIterError::TooFewItems) => {}
            Ok(_) | Err(BoxedSliceFromIterError::Oom(_)) => unreachable!(),
        }

        assert!(a_dropped.get());
        assert!(b_dropped.get());
        assert!(c_dropped.get());
    }

    #[test]
    fn new_boxed_slice_from_iter_with_too_many_elems() {
        let (a_dropped, a) = SetFlagOnDrop::new();
        let (b_dropped, b) = SetFlagOnDrop::new();
        let (c_dropped, c) = SetFlagOnDrop::new();

        let slice = new_boxed_slice_from_iter(2, [a, b, c]).unwrap();

        assert!(!a_dropped.get());
        assert!(!b_dropped.get());
        assert!(c_dropped.get());

        drop(slice);

        assert!(a_dropped.get());
        assert!(b_dropped.get());
        assert!(c_dropped.get());
    }
}
