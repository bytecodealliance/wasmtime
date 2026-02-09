use crate::{
    alloc::{TryClone, str_ptr_from_raw_parts, try_realloc},
    error::OutOfMemory,
};
use core::{fmt, mem, ops};
use std_alloc::{alloc::Layout, boxed::Box, string as inner};

/// A newtype wrapper around [`std::string::String`] that only exposes
/// fallible-allocation methods.
#[derive(Default)]
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

impl serde::ser::Serialize for String {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl<'de> serde::de::Deserialize<'de> for String {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = String;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a `wasmtime_core::alloc::String` str")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut s = String::new();
                s.reserve_exact(v.len()).map_err(|oom| E::custom(oom))?;
                s.push_str(v).expect("reserved capacity");
                Ok(s)
            }
        }

        // NB: do not use `deserialize_string` as that eagerly allocates the
        // `String` and does not give us a chance to handle OOM. Instead, use
        // `deserialize_str` which passes the visitor the borrowed `str`, giving
        // us a chance to fallibly allocate space.
        deserializer.deserialize_str(Visitor)
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

    /// Same as [`std::string::String::capacity`].
    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
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

    /// Same as [`std::string::String::into_raw_parts`].
    pub fn into_raw_parts(mut self) -> (*mut u8, usize, usize) {
        // NB: Can't use `String::into_raw_parts` until our MSRV is >= 1.93.
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

    /// Same as [`std::string::String::from_raw_parts`].
    pub unsafe fn from_raw_parts(buf: *mut u8, length: usize, capacity: usize) -> Self {
        Self {
            // Safety: Same as our unsafe contract.
            inner: unsafe { inner::String::from_raw_parts(buf, length, capacity) },
        }
    }

    /// Same as [`std::string::String::shrink_to_fit`] but returns an error on
    /// allocation failure.
    pub fn shrink_to_fit(&mut self) -> Result<(), OutOfMemory> {
        // If our length is already equal to our capacity, then there is nothing
        // to shrink.
        if self.len() == self.capacity() {
            return Ok(());
        }

        // `realloc` requires a non-zero original layout as well as a non-zero
        // destination layout, so this guard ensures that the sizes below are
        // all nonzero. This handles a couple cases:
        //
        // * If `len == cap == 0` then no allocation has ever been made.
        // * If `len == 0` and `cap != 0` then this function effectively frees
        //   the memory.
        //
        // In both of these cases delegate to the standard library's
        // `shrink_to_fit` which is guaranteed to not perform a `realloc`.
        if self.is_empty() {
            self.inner.shrink_to_fit();
            return Ok(());
        }

        let (ptr, len, cap) = mem::take(self).into_raw_parts();
        debug_assert!(!ptr.is_null());
        debug_assert!(len > 0);
        debug_assert!(cap > len);
        let old_layout = Layout::array::<u8>(cap).unwrap();
        debug_assert_eq!(old_layout.size(), cap);
        let new_layout = Layout::array::<u8>(len).unwrap();
        debug_assert_eq!(old_layout.align(), new_layout.align());
        debug_assert_eq!(new_layout.size(), len);

        // SAFETY: `ptr` was previously allocated in the global allocator,
        // `layout` has a nonzero size and matches the current allocation of
        // `ptr`, `len` is nonzero, and `len` is a valid array size
        // for `len` elements given its constructor.
        let result = unsafe { try_realloc(ptr, old_layout, len) };

        match result {
            Ok(ptr) => {
                // SAFETY: `result` is allocated with the global allocator and
                // has room for exactly `[u8; len]`.
                *self = unsafe { Self::from_raw_parts(ptr.as_ptr(), len, len) };
                Ok(())
            }
            Err(oom) => {
                // SAFETY: If reallocation fails then it's guaranteed that the
                // original allocation is not tampered with, so it's safe to
                // reassemble the original vector.
                *self = unsafe { Self::from_raw_parts(ptr, len, cap) };
                Err(oom)
            }
        }
    }

    /// Same as [`std::string::String::into_boxed_str`] but returns an error on
    /// allocation failure.
    pub fn into_boxed_str(mut self) -> Result<Box<str>, OutOfMemory> {
        self.shrink_to_fit()?;

        let (ptr, len, cap) = self.into_raw_parts();
        debug_assert_eq!(len, cap);
        let ptr = str_ptr_from_raw_parts(ptr, len);

        // SAFETY: The `ptr` is allocated with the global allocator and points
        // to a valid block of utf8.
        let boxed = unsafe { Box::from_raw(ptr) };

        Ok(boxed)
    }
}
