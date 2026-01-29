use crate::error::OutOfMemory;
use core::{
    fmt, mem,
    ops::{Deref, DerefMut, Index, IndexMut},
};
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
        let ptr = self.as_mut_ptr();
        let len = self.len();
        let cap = self.capacity();
        mem::forget(self);
        (ptr, len, cap)
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
