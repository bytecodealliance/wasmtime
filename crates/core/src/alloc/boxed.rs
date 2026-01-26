use super::{TryNew, try_alloc};
use crate::error::OutOfMemory;
use core::{
    alloc::Layout,
    mem::{self, MaybeUninit},
};
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
        .map_err(|_| OutOfMemory::new(len.saturating_mul(mem::size_of::<T>())))?;

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

/// RAII guard to handle dropping the initialized elements of the boxed
/// slice in the cases where we get too few items or the iterator panics.
struct DropGuard<T> {
    boxed: Box<[MaybeUninit<T>]>,
    init_len: usize,
}

impl<T> Drop for DropGuard<T> {
    fn drop(&mut self) {
        debug_assert!(self.init_len <= self.boxed.len());

        if !mem::needs_drop::<T>() {
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

impl<T> DropGuard<T> {
    fn new(len: usize) -> Result<Self, OutOfMemory> {
        Ok(DropGuard {
            boxed: new_uninit_boxed_slice(len)?,
            init_len: 0,
        })
    }

    /// Finish this guard and take its boxed slice out.
    fn finish(mut self) -> Box<[MaybeUninit<T>]> {
        self.init_len = 0;
        let boxed = mem::take(&mut self.boxed);
        mem::forget(self);
        boxed
    }

    /// Reallocate this guard's boxed slice such that its length doubles.
    fn double(&mut self) -> Result<(), OutOfMemory> {
        let new_len = self.boxed.len().saturating_mul(2);
        let new_len = core::cmp::max(new_len, 4);
        self.realloc(new_len)
    }

    /// Shrink this guard's boxed slice to exactly the initalized length.
    fn shrink_to_fit(&mut self) -> Result<(), OutOfMemory> {
        self.realloc(self.init_len)
    }

    /// Reallocate this guard's boxed slice to `new_len`.
    ///
    /// The number of initialized elements will remain the same, and `new_len`
    /// must be greater than or equal to the initialized length.
    fn realloc(&mut self, new_len: usize) -> Result<(), OutOfMemory> {
        assert!(self.init_len <= new_len);

        if new_len == self.boxed.len() {
            return Ok(());
        }

        let old_layout = Layout::array::<T>(self.boxed.len())
            .expect("already have an allocation with this layout so should be able to recreate it");
        let new_layout = Layout::array::<T>(new_len)
            .map_err(|_| OutOfMemory::new(mem::size_of::<T>().saturating_mul(new_len)))?;
        debug_assert_eq!(old_layout.align(), new_layout.align());

        // Temporarily take the boxed slice out of `self` and reset
        // `self.init_len` so that if we panic during reallocation, we never get
        // mixed up and see invalid state on `self` inside our `Drop`
        // implementation.
        let init_len = mem::take(&mut self.init_len);
        let boxed = mem::take(&mut self.boxed);
        let ptr = Box::into_raw(boxed);

        // Handle zero-sized reallocations, since the global `realloc` function
        // does not.
        if new_layout.size() == 0 {
            debug_assert!(mem::size_of::<T>() == 0 || new_len == 0);
            if new_len == 0 {
                debug_assert_eq!(self.boxed.len(), 0);
                debug_assert_eq!(self.init_len, 0);
            } else {
                self.boxed = Box::new_uninit_slice(new_len);
                self.init_len = init_len;
            }
            return Ok(());
        }

        // Safety: `ptr` was allocated by the global allocator, its memory block
        // is described by `old_layout`, the new size is non-zero, and the new
        // size will not overflow `isize::MAX` when rounded up to the layout's
        // alignment (this is checked in the fallible construction of
        // `new_layout`).
        let new_ptr =
            unsafe { std_alloc::alloc::realloc(ptr.cast::<u8>(), old_layout, new_layout.size()) };

        // Update `self` based on whether the allocation succeeded or not,
        // either inserting in the new slice or replacing the old slice.
        if new_ptr.is_null() {
            // Safety: The allocation failed so we retain ownership of `ptr`,
            // which was a valid boxed slice and we can safely make it a boxed
            // slice again. The block's contents were not modified, so the old
            // `init_len` remains valid.
            self.boxed = unsafe { Box::from_raw(ptr) };
            self.init_len = init_len;
            Err(OutOfMemory::new(new_layout.size()))
        } else {
            let new_ptr = new_ptr.cast::<MaybeUninit<T>>();
            let new_ptr = core::ptr::slice_from_raw_parts_mut(new_ptr, new_len);
            // Safety: The allocation succeeded, `new_ptr` was allocated by the
            // global allocator and points to a valid boxed slice of length
            // `new_len`, and the old allocation's contents were copied over to
            // the new allocation so the old `init_len` remains valid.
            self.boxed = unsafe { Box::from_raw(new_ptr) };
            self.init_len = init_len;
            Ok(())
        }
    }
}

/// An error returned by [`new_boxed_slice_from_iter`].
#[derive(Debug)]
pub enum BoxedSliceFromIterWithLenError {
    /// The iterator did not yield enough items to fill the boxed slice.
    TooFewItems,
    /// Failed to allocate space for the boxed slice.
    Oom(OutOfMemory),
}

impl From<OutOfMemory> for BoxedSliceFromIterWithLenError {
    fn from(oom: OutOfMemory) -> Self {
        Self::Oom(oom)
    }
}

impl core::fmt::Display for BoxedSliceFromIterWithLenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewItems => {
                f.write_str("The iterator did not yield enough items to fill the boxed slice")
            }
            Self::Oom(_) => f.write_str("Failed to allocate space for the boxed slice"),
        }
    }
}

impl core::error::Error for BoxedSliceFromIterWithLenError {
    fn cause(&self) -> Option<&dyn core::error::Error> {
        match self {
            Self::TooFewItems => None,
            Self::Oom(oom) => Some(oom),
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
pub fn new_boxed_slice_from_iter_with_len<T>(
    len: usize,
    iter: impl IntoIterator<Item = T>,
) -> Result<Box<[T]>, BoxedSliceFromIterWithLenError> {
    let mut guard = DropGuard::new(len)?;
    assert_eq!(len, guard.boxed.len());

    for (i, elem) in iter.into_iter().enumerate().take(len) {
        debug_assert!(i < len);
        debug_assert_eq!(guard.init_len, i);
        guard.boxed[i].write(elem);
        guard.init_len += 1;
    }

    debug_assert!(guard.init_len <= guard.boxed.len());

    if guard.init_len < guard.boxed.len() {
        return Err(BoxedSliceFromIterWithLenError::TooFewItems);
    }

    debug_assert_eq!(guard.init_len, guard.boxed.len());
    let boxed = guard.finish();

    // Safety: we initialized all elements.
    let boxed = unsafe { boxed.assume_init() };

    Ok(boxed)
}

/// An error returned by [`new_boxed_slice_from_fallible_iter`].
#[derive(Debug)]
pub enum BoxedSliceFromFallibleIterError<E> {
    /// The fallible iterator produced an error.
    IterError(E),
    /// Failed to allocate space for the boxed slice.
    Oom(OutOfMemory),
}

impl<E> From<OutOfMemory> for BoxedSliceFromFallibleIterError<E> {
    fn from(oom: OutOfMemory) -> Self {
        Self::Oom(oom)
    }
}

impl<E> core::fmt::Display for BoxedSliceFromFallibleIterError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IterError(_) => f.write_str("The fallible iterator produced an error"),
            Self::Oom(_) => f.write_str("Failed to allocate space for the boxed slice"),
        }
    }
}

impl<E> core::error::Error for BoxedSliceFromFallibleIterError<E>
where
    E: core::error::Error,
{
    fn cause(&self) -> Option<&dyn core::error::Error> {
        match self {
            Self::IterError(e) => Some(e),
            Self::Oom(oom) => Some(oom),
        }
    }
}

impl BoxedSliceFromFallibleIterError<OutOfMemory> {
    /// Flatten this error into its inner OOM.
    pub fn flatten(self) -> OutOfMemory {
        match self {
            Self::IterError(oom) | Self::Oom(oom) => oom,
        }
    }
}

/// Create a `Box<[T]>` from the given iterator's `Result<T, E>` items.
///
/// Returns an error on allocation failure or if an iterator item is an `Err`.
pub fn new_boxed_slice_from_fallible_iter<T, E>(
    iter: impl IntoIterator<Item = Result<T, E>>,
) -> Result<Box<[T]>, BoxedSliceFromFallibleIterError<E>> {
    let iter = iter.into_iter();

    let (min, max) = iter.size_hint();
    let len = max.unwrap_or_else(|| min);

    let mut guard = DropGuard::new(len)?;
    assert_eq!(len, guard.boxed.len());

    for (i, result) in iter.enumerate() {
        debug_assert_eq!(guard.init_len, i);
        let elem = match result {
            Ok(x) => x,
            Err(e) => return Err(BoxedSliceFromFallibleIterError::IterError(e)),
        };

        if i >= guard.boxed.len() {
            guard.double()?;
        }
        debug_assert!(i < guard.boxed.len());
        guard.boxed[i].write(elem);
        guard.init_len += 1;
    }

    debug_assert!(guard.init_len <= guard.boxed.len());
    guard.shrink_to_fit()?;
    debug_assert_eq!(guard.init_len, guard.boxed.len());

    // Take the boxed slice out of the guard.
    let boxed = guard.finish();

    // Safety: we initialized all elements.
    let boxed = unsafe { boxed.assume_init() };

    Ok(boxed)
}

/// Create a `Box<[T]>` from the given iterator's elements.
///
/// Returns an error on allocation failure.
pub fn new_boxed_slice_from_iter<T>(
    iter: impl IntoIterator<Item = T>,
) -> Result<Box<[T]>, OutOfMemory> {
    let iter = iter
        .into_iter()
        .map(Result::<T, core::convert::Infallible>::Ok);
    new_boxed_slice_from_fallible_iter(iter).map_err(|e| match e {
        BoxedSliceFromFallibleIterError::Oom(oom) => oom,
        BoxedSliceFromFallibleIterError::IterError(_) => unreachable!(),
    })
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
    fn new_boxed_slice_from_iter_with_len_smoke_test() {
        let slice = new_boxed_slice_from_iter_with_len(3, [42, 36, 1337]).unwrap();
        assert_eq!(&*slice, &[42, 36, 1337]);
    }

    #[test]
    fn new_boxed_slice_from_iter_with_len_with_too_few_elems() {
        let (a_dropped, a) = SetFlagOnDrop::new();
        let (b_dropped, b) = SetFlagOnDrop::new();
        let (c_dropped, c) = SetFlagOnDrop::new();

        match new_boxed_slice_from_iter_with_len(4, [a, b, c]) {
            Err(BoxedSliceFromIterWithLenError::TooFewItems) => {}
            Ok(_) | Err(BoxedSliceFromIterWithLenError::Oom(_)) => unreachable!(),
        }

        assert!(a_dropped.get());
        assert!(b_dropped.get());
        assert!(c_dropped.get());
    }

    #[test]
    fn new_boxed_slice_from_iter_with_len_with_too_many_elems() {
        let (a_dropped, a) = SetFlagOnDrop::new();
        let (b_dropped, b) = SetFlagOnDrop::new();
        let (c_dropped, c) = SetFlagOnDrop::new();

        let slice = new_boxed_slice_from_iter_with_len(2, [a, b, c]).unwrap();

        assert!(!a_dropped.get());
        assert!(!b_dropped.get());
        assert!(c_dropped.get());

        drop(slice);

        assert!(a_dropped.get());
        assert!(b_dropped.get());
        assert!(c_dropped.get());
    }

    #[test]
    fn new_boxed_slice_from_iter_smoke_test() {
        let slice = new_boxed_slice_from_iter([10, 20, 30]).unwrap();
        assert_eq!(&*slice, &[10, 20, 30]);
    }

    #[test]
    fn new_boxed_slice_from_fallible_iter_smoke_test() {
        let slice =
            new_boxed_slice_from_fallible_iter::<_, &str>([Ok(10), Ok(20), Ok(30)]).unwrap();
        assert_eq!(&*slice, &[10, 20, 30]);
    }

    #[test]
    fn new_boxed_slice_from_fallible_iter_error() {
        let result = new_boxed_slice_from_fallible_iter::<_, u32>([Ok(10), Ok(20), Err(30)]);
        let Err(BoxedSliceFromFallibleIterError::IterError(err)) = result else {
            panic!("unexpected result: {result:?}");
        };
        assert_eq!(err, 30);
    }
}
