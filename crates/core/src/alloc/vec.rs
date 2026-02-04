use crate::alloc::{TryClone, try_realloc};
use crate::error::OutOfMemory;
use core::{
    fmt, mem,
    ops::{Deref, DerefMut, Index, IndexMut},
};
use std_alloc::alloc::Layout;
use std_alloc::boxed::Box;
use std_alloc::vec::Vec as StdVec;

/// Like `std::vec::Vec` but all methods that allocate force handling allocation
/// failure.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vec<T> {
    inner: StdVec<T>,
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T> TryClone for Vec<T>
where
    T: TryClone,
{
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        let mut v = Vec::with_capacity(self.len())?;
        for x in self {
            v.push(x.try_clone()?).expect("reserved capacity");
        }
        Ok(v)
    }
}

impl<T> Vec<T> {
    /// Same as [`std::vec::Vec::new`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Same as [`std::vec::Vec::with_capacity`] but returns an error on
    /// allocation failure.
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut v = Self::new();
        v.reserve(capacity)?;
        Ok(v)
    }

    /// Same as [`std::vec::Vec::reserve`] but returns an error on allocation
    /// failure.
    pub fn reserve(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve(additional).map_err(|_| {
            OutOfMemory::new(
                self.len()
                    .saturating_add(additional)
                    .saturating_mul(mem::size_of::<T>()),
            )
        })
    }

    /// Same as [`std::vec::Vec::reserve_exact`] but returns an error on allocation
    /// failure.
    pub fn reserve_exact(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner
            .try_reserve_exact(additional)
            .map_err(|_| OutOfMemory::new(self.len().saturating_add(additional)))
    }

    /// Same as [`std::vec::Vec::len`].
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Same as [`std::vec::Vec::capacity`].
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Same as [`std::vec::Vec::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`std::vec::Vec::push`] but returns an error on allocation
    /// failure.
    pub fn push(&mut self, value: T) -> Result<(), OutOfMemory> {
        self.reserve(1)?;
        self.inner.push(value);
        Ok(())
    }

    /// Same as [`std::vec::Vec::pop`].
    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }

    /// Same as [`std::vec::Vec::into_raw_parts`].
    pub fn into_raw_parts(mut self) -> (*mut T, usize, usize) {
        // NB: Can't use `Vec::into_raw_parts` until our MSRV is >= 1.93.
        #[cfg(not(miri))]
        {
            let ptr = self.as_mut_ptr();
            let len = self.len();
            let cap = self.capacity();
            mem::forget(self);
            (ptr, len, cap)
        }
        // NB: Miri requires using `into_raw_parts`, but always run on nightly,
        // so it's fine to use there.
        #[cfg(miri)]
        {
            let _ = &mut self;
            self.inner.into_raw_parts()
        }
    }

    /// Same as [`std::vec::Vec::from_raw_parts`].
    pub unsafe fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize) -> Self {
        Vec {
            // Safety: Same as our unsafe contract.
            inner: unsafe { StdVec::from_raw_parts(ptr, length, capacity) },
        }
    }

    /// Same as [`std::vec::Vec::drain`].
    pub fn drain<R>(&mut self, range: R) -> std_alloc::vec::Drain<'_, T>
    where
        R: core::ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    /// Same as [`std::vec::Vec::into_boxed_slice`].
    pub fn into_boxed_slice(self) -> Result<Box<[T]>, OutOfMemory> {
        // `realloc` requires a non-zero original layout as well as a non-zero
        // destination layout, so this guard ensures that the sizes below are
        // all nonzero. This handles a few case:
        //
        // * If `len == cap == 0` then no allocation has ever been made.
        // * If `len == 0` and `cap != 0` then this function effectively frees
        //   the memory.
        // * If `T` is a zero-sized type then nothing's been allocated either.
        //
        // In all of these cases delegate to the standard library's
        // `into_boxed_slice` which is guaranteed to not perform a `realloc`.
        if self.is_empty() || mem::size_of::<T>() == 0 {
            return Ok(self.inner.into_boxed_slice());
        }

        let (ptr, len, cap) = self.into_raw_parts();
        let layout = Layout::array::<T>(cap).unwrap();
        let new_len = Layout::array::<T>(len).unwrap().size();

        // SAFETY: `ptr` was previously allocated in the global allocator,
        // `layout` has a nonzero size and matches the current allocation of
        // `ptr`, `new_size` is nonzero, and `new_size` is a valid array size
        // for `len` elements given its constructor.
        let result = unsafe { try_realloc(ptr.cast(), layout, new_len) };

        match result {
            Ok(ptr) => {
                // SAFETY: `result` is allocated with the global allocator with
                // an appropriate size/align to create this `Box` with.
                unsafe {
                    Ok(Box::from_raw(core::ptr::slice_from_raw_parts_mut(
                        ptr.as_ptr().cast(),
                        len,
                    )))
                }
            }
            Err(oom) => {
                // SAFETY: If reallocation fails then it's guaranteed that the
                // original allocation is not tampered with, so it's safe to
                // reassemble it back into the original vector.
                unsafe {
                    let _ = Vec::from_raw_parts(ptr, len, cap);
                }
                Err(oom)
            }
        }
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Index<usize> for Vec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T> IndexMut<usize> for Vec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = std_alloc::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Vec<T> {
    type Item = &'a T;

    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (**self).iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Vec<T> {
    type Item = &'a mut T;

    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        (**self).iter_mut()
    }
}

impl<T> From<Box<[T]>> for Vec<T> {
    fn from(boxed_slice: Box<[T]>) -> Self {
        Vec {
            inner: StdVec::from(boxed_slice),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Vec;
    use crate::error::OutOfMemory;

    #[test]
    fn test_into_boxed_slice() -> Result<(), OutOfMemory> {
        assert_eq!(*Vec::<i32>::new().into_boxed_slice()?, []);

        let mut vec = Vec::new();
        vec.push(1)?;
        assert_eq!(*vec.into_boxed_slice()?, [1]);

        let mut vec = Vec::with_capacity(2)?;
        vec.push(1)?;
        assert_eq!(*vec.into_boxed_slice()?, [1]);

        let mut vec = Vec::with_capacity(2)?;
        vec.push(1_u128)?;
        assert_eq!(*vec.into_boxed_slice()?, [1]);

        assert_eq!(*Vec::<()>::new().into_boxed_slice()?, []);

        let mut vec = Vec::new();
        vec.push(())?;
        assert_eq!(*vec.into_boxed_slice()?, [()]);

        let vec = Vec::<i32>::with_capacity(2)?;
        assert_eq!(*vec.into_boxed_slice()?, []);
        Ok(())
    }
}
