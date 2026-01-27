use super::{TryNew, Vec, try_alloc};
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

/// Allocate a new `Box<[MaybeUninit<T>]>` of the given length with
/// uninitialized contents, returning `Err(OutOfMemory)` on allocation failure.
///
/// You can initialize the resulting boxed slice with
/// [`wasmtime_core::alloc::boxed_slice_write_iter`].
pub fn new_uninit_boxed_slice<T>(len: usize) -> Result<Box<[MaybeUninit<T>]>, OutOfMemory> {
    let layout = Layout::array::<MaybeUninit<T>>(len)
        .map_err(|_| OutOfMemory::new(mem::size_of::<T>().saturating_mul(len)))?;

    if layout.size() == 0 {
        // NB: no actual allocation takes place when boxing zero-sized
        // types.
        return Ok(Box::new_uninit_slice(len));
    }

    // Safety: layout size is non-zero.
    let ptr = unsafe { try_alloc(layout)? };

    let ptr = ptr.cast::<MaybeUninit<T>>().as_ptr();
    let ptr = core::ptr::slice_from_raw_parts_mut(ptr, len);

    // Safety: The pointer's memory block was allocated by the global allocator
    // and holds room for `[T; len]`.
    Ok(unsafe { Box::from_raw(ptr) })
}

use boxed_slice_builder::BoxedSliceBuilder;
mod boxed_slice_builder {
    use super::*;

    /// Builder for constructing and initalizing a boxed slice.
    ///
    /// Also acts as an RAII guard to handle dropping the already-initialized
    /// elements when we get too few items or an iterator panics during
    /// construction.
    pub struct BoxedSliceBuilder<T> {
        vec: Vec<T>,
    }

    impl<T> BoxedSliceBuilder<T> {
        pub fn new(len: usize) -> Result<Self, OutOfMemory> {
            let mut vec = Vec::new();
            vec.reserve_exact(len)?;
            Ok(Self { vec })
        }

        pub fn from_boxed_slice(boxed: Box<[MaybeUninit<T>]>) -> Self {
            let len = boxed.len();
            let ptr = Box::into_raw(boxed);
            let ptr = ptr.cast::<T>();
            // Safety: the pointer was allocated by the global allocator and is
            // valid for `[T; len]` since it was a boxed slice.
            let vec = unsafe { Vec::from_raw_parts(ptr, 0, len) };
            Self { vec }
        }

        pub fn init_len(&self) -> usize {
            self.vec.len()
        }

        pub fn capacity(&self) -> usize {
            self.vec.capacity()
        }

        pub fn push(&mut self, value: T) -> Result<(), OutOfMemory> {
            self.vec.push(value)
        }

        /// Finish this builder and take its boxed slice out.
        ///
        /// Panics if `self.init_len() != self.capacity()`. Call
        /// `self.shrink_to_fit()` if necessary.
        pub fn finish(mut self) -> Box<[T]> {
            assert_eq!(self.init_len(), self.capacity());
            let vec = mem::take(&mut self.vec);
            mem::forget(self);
            let (ptr, len, cap) = vec.into_raw_parts();
            debug_assert_eq!(len, cap);
            let ptr = core::ptr::slice_from_raw_parts_mut(ptr, len);
            unsafe { Box::from_raw(ptr) }
        }

        /// Shrink this builder's allocation such that `self.init_len() ==
        /// self.capacity()`.
        pub fn shrink_to_fit(&mut self) -> Result<(), OutOfMemory> {
            if self.init_len() == self.capacity() {
                return Ok(());
            }

            let len = self.init_len();
            let cap = self.capacity();
            let vec = mem::take(&mut self.vec);

            let old_layout = Layout::array::<T>(cap).expect(
                "already have an allocation with this layout so should be able to recreate it",
            );
            let new_layout = Layout::array::<T>(len)
                .expect("if `cap` is fine for an array layout, then `len` must be as well");
            debug_assert_eq!(old_layout.align(), new_layout.align());

            // Handle zero-sized reallocations, since the global `realloc` function
            // does not.
            if new_layout.size() == 0 {
                debug_assert!(mem::size_of::<T>() == 0 || len == 0);
                if len == 0 {
                    debug_assert_eq!(self.capacity(), 0);
                    debug_assert_eq!(self.init_len(), 0);
                } else {
                    debug_assert_eq!(mem::size_of::<T>(), 0);
                    let ptr = core::ptr::dangling_mut::<T>();
                    debug_assert!(!ptr.is_null());
                    debug_assert!(ptr.is_aligned());
                    // Safety: T's dangling pointer is always non-null and aligned.
                    self.vec = unsafe { Vec::from_raw_parts(ptr, len, len) };
                }
                debug_assert_eq!(self.capacity(), self.init_len());
                return Ok(());
            }

            let (ptr, _len, _cap) = vec.into_raw_parts();
            debug_assert_eq!(len, _len);
            debug_assert_eq!(cap, _cap);

            // Safety: `ptr` was allocated by the global allocator, its memory block
            // is described by `old_layout`, the new size is non-zero, and the new
            // size will not overflow `isize::MAX` when rounded up to the layout's
            // alignment (this is checked in the construction of `new_layout`).
            let new_ptr = unsafe {
                std_alloc::alloc::realloc(ptr.cast::<u8>(), old_layout, new_layout.size())
            };

            // Update `self` based on whether the reallocation succeeded or not,
            // either inserting the new vec or reconstructing and replacing the
            // old one.
            if new_ptr.is_null() {
                // Safety: The allocation failed so we retain ownership of `ptr`,
                // which was a valid vec and we can safely make it a vec again.
                self.vec = unsafe { Vec::from_raw_parts(ptr, len, cap) };
                Err(OutOfMemory::new(new_layout.size()))
            } else {
                let new_ptr = new_ptr.cast::<T>();
                // Safety: The allocation succeeded, `new_ptr` was reallocated by
                // the global allocator and points to a valid boxed slice of length
                // `len`.
                self.vec = unsafe { Vec::from_raw_parts(new_ptr, len, len) };
                debug_assert_eq!(self.capacity(), self.init_len());
                Ok(())
            }
        }
    }
}

/// An error returned when an iterator yields too few items to fully initialize
/// a `Box<[MaybeUninit<T>]>`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct TooFewItems;

impl core::fmt::Display for TooFewItems {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("iterator yielded too few items to fully initialize boxed slice")
    }
}

impl core::error::Error for TooFewItems {}

/// An error returned by [`new_boxed_slice_from_iter`].
#[derive(Debug)]
pub enum TooFewItemsOrOom {
    /// The iterator did not yield enough items to fill the boxed slice.
    TooFewItems(TooFewItems),
    /// Failed to allocate space for the boxed slice.
    Oom(OutOfMemory),
}

impl TooFewItemsOrOom {
    /// Unwrap the inner `OutOfMemory` error, or panic if this is a different
    /// error variant.
    pub fn unwrap_oom(&self) -> OutOfMemory {
        match self {
            TooFewItemsOrOom::TooFewItems(_) => panic!("`unwrap_oom` on non-OOM error"),
            TooFewItemsOrOom::Oom(oom) => *oom,
        }
    }
}

impl From<TooFewItems> for TooFewItemsOrOom {
    fn from(e: TooFewItems) -> Self {
        Self::TooFewItems(e)
    }
}

impl From<OutOfMemory> for TooFewItemsOrOom {
    fn from(oom: OutOfMemory) -> Self {
        Self::Oom(oom)
    }
}

impl core::fmt::Display for TooFewItemsOrOom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewItems(_) => {
                f.write_str("The iterator did not yield enough items to fill the boxed slice")
            }
            Self::Oom(_) => f.write_str("Failed to allocate space for the boxed slice"),
        }
    }
}

impl core::error::Error for TooFewItemsOrOom {
    fn cause(&self) -> Option<&dyn core::error::Error> {
        match self {
            Self::TooFewItems(e) => Some(e),
            Self::Oom(oom) => Some(oom),
        }
    }
}

/// Initialize a `Box<[MaybeUninit<T>]>` slice by writing the elements of the
/// given iterator into it.
pub fn boxed_slice_write_iter<T>(
    boxed: Box<[MaybeUninit<T>]>,
    iter: impl IntoIterator<Item = T>,
) -> Result<Box<[T]>, TooFewItems> {
    let len = boxed.len();
    let builder = BoxedSliceBuilder::from_boxed_slice(boxed);
    assert_eq!(len, builder.capacity());
    write_iter_into_builder(builder, iter)
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
) -> Result<Box<[T]>, TooFewItemsOrOom> {
    let builder = BoxedSliceBuilder::new(len)?;
    assert_eq!(len, builder.capacity());
    let boxed = write_iter_into_builder(builder, iter)?;
    Ok(boxed)
}

fn write_iter_into_builder<T>(
    mut builder: BoxedSliceBuilder<T>,
    iter: impl IntoIterator<Item = T>,
) -> Result<Box<[T]>, TooFewItems> {
    let len = builder.capacity();

    for elem in iter.into_iter().take(len) {
        builder.push(elem).expect("reserved capacity");
    }

    if builder.init_len() < builder.capacity() {
        return Err(TooFewItems);
    }

    debug_assert_eq!(builder.init_len(), builder.capacity());
    Ok(builder.finish())
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

    let mut builder = BoxedSliceBuilder::new(len)?;
    assert_eq!(len, builder.capacity());

    for result in iter {
        let elem = result.map_err(BoxedSliceFromFallibleIterError::IterError)?;
        builder.push(elem)?;
    }

    debug_assert!(builder.init_len() <= builder.capacity());
    builder.shrink_to_fit()?;
    debug_assert_eq!(builder.init_len(), builder.capacity());

    Ok(builder.finish())
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
            Err(TooFewItemsOrOom::TooFewItems(_)) => {}
            Ok(_) | Err(TooFewItemsOrOom::Oom(_)) => unreachable!(),
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

    #[test]
    fn new_uninit_boxed_slice_smoke_test() {
        let slice = new_uninit_boxed_slice::<u32>(5).unwrap();
        assert_eq!(slice.len(), 5);
    }

    #[test]
    fn boxed_slice_write_iter_smoke_test() {
        let uninit = new_uninit_boxed_slice(3).unwrap();
        let init = boxed_slice_write_iter(uninit, [10, 20, 30]).unwrap();
        assert_eq!(&*init, &[10, 20, 30]);
    }

    #[test]
    fn boxed_slice_write_iter_with_too_few_elems() {
        let (a_dropped, a) = SetFlagOnDrop::new();
        let (b_dropped, b) = SetFlagOnDrop::new();
        let (c_dropped, c) = SetFlagOnDrop::new();

        let uninit = new_uninit_boxed_slice(4).unwrap();
        match boxed_slice_write_iter(uninit, [a, b, c]) {
            Err(_) => {}
            Ok(_) => unreachable!(),
        }

        assert!(a_dropped.get());
        assert!(b_dropped.get());
        assert!(c_dropped.get());
    }

    #[test]
    fn boxed_slice_write_iter_with_too_many_elems() {
        let (a_dropped, a) = SetFlagOnDrop::new();
        let (b_dropped, b) = SetFlagOnDrop::new();
        let (c_dropped, c) = SetFlagOnDrop::new();

        let uninit = new_uninit_boxed_slice(2).unwrap();
        let slice = boxed_slice_write_iter(uninit, [a, b, c]).unwrap();

        assert!(!a_dropped.get());
        assert!(!b_dropped.get());
        assert!(c_dropped.get());

        drop(slice);

        assert!(a_dropped.get());
        assert!(b_dropped.get());
        assert!(c_dropped.get());
    }
}
