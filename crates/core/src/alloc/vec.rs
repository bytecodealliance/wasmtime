use crate::error::OutOfMemory;
use core::{
    fmt,
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
                    .saturating_mul(core::mem::size_of::<T>()),
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
