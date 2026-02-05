use super::TryClone;
use crate::error::OutOfMemory;
use core::{fmt, ops};
use std_alloc::string as inner;

/// A newtype wrapper around [`std::string::String`] that only exposes
/// fallible-allocation methods.
pub struct String {
    inner: inner::String,
}

impl TryClone for String {
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        let mut s = Self::new();
        s.push_str(self)?;
        Ok(s)
    }
}

impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl ops::Deref for String {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ops::DerefMut for String {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<inner::String> for String {
    #[inline]
    fn from(inner: inner::String) -> Self {
        Self { inner }
    }
}

impl String {
    /// Same as [`std::string::String::new`].
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: inner::String::new(),
        }
    }

    /// Same as [`std::string::String::with_capacity`] but returns an error on
    /// allocation failure.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut s = Self::new();
        s.reserve(capacity)?;
        Ok(s)
    }

    /// Same as [`std::string::String::reserve`] but returns an error on
    /// allocation failure.
    #[inline]
    pub fn reserve(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner
            .try_reserve(additional)
            .map_err(|_| OutOfMemory::new(self.len().saturating_add(additional)))
    }

    /// Same as [`std::string::String::reserve_exact`] but returns an error on
    /// allocation failure.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner
            .try_reserve_exact(additional)
            .map_err(|_| OutOfMemory::new(self.len().saturating_add(additional)))
    }

    /// Same as [`std::string::String::push`] but returns an error on allocation
    /// failure.
    #[inline]
    pub fn push(&mut self, c: char) -> Result<(), OutOfMemory> {
        self.reserve(c.len_utf8())?;
        self.inner.push(c);
        Ok(())
    }

    /// Same as [`std::string::String::push_str`] but returns an error on
    /// allocation failure.
    #[inline]
    pub fn push_str(&mut self, s: &str) -> Result<(), OutOfMemory> {
        self.reserve(s.len())?;
        self.inner.push_str(s);
        Ok(())
    }
}
