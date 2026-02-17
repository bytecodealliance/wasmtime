use crate::alloc::{TryClone, try_realloc};
use crate::error::OutOfMemory;
use core::{
    cmp::Ordering,
    fmt,
    marker::PhantomData,
    mem,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Index, IndexMut},
    slice::SliceIndex,
};
use serde::ser::SerializeSeq;
use std_alloc::alloc::Layout;
use std_alloc::boxed::Box;
use std_alloc::vec::Vec as StdVec;

/// Same as the [`std::vec!`] macro but returns an error on allocation failure.
#[macro_export]
macro_rules! vec {
    ( $( $elem:expr ),* ) => {{
        let len = $crate::private_len!( $( $elem ),* );
        $crate::alloc::Vec::with_capacity(len).and_then(|mut v| {
            $( v.push($elem)?; )*
            let _ = &mut v;
            Ok(v)
        })
    }};

    ( $elem:expr; $len:expr ) => {{
        let len: usize = $len;
        if let Some(len) = ::core::num::NonZeroUsize::new(len) {
            let elem = $elem;
            $crate::alloc::Vec::from_elem(elem, len)
        } else {
            Ok($crate::alloc::Vec::new())
        }
    }};

}

// Only for use by the `vec!` macro.
#[doc(hidden)]
#[macro_export]
macro_rules! private_len {
    ( ) => { 0 };
    ( $e:expr $( , $es:expr )* ) => { 1 + $crate::private_len!( $( $es ),* ) };
}

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
    pub const fn new() -> Self {
        Self {
            inner: StdVec::new(),
        }
    }

    /// Same as [`std::vec::Vec::with_capacity`] but returns an error on
    /// allocation failure.
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut v = Self::new();
        v.reserve(capacity)?;
        Ok(v)
    }

    // For use with the `vec!` macro.
    #[doc(hidden)]
    #[inline]
    pub fn from_elem(elem: T, len: NonZeroUsize) -> Result<Self, OutOfMemory>
    where
        T: TryClone,
    {
        let mut v = Self::with_capacity(len.get())?;

        // Minimize calls to `TryClone` by always pushing `elem` itself as the
        // last element.
        for _ in 0..len.get() - 1 {
            v.push(elem.try_clone()?)?;
        }
        v.push(elem)?;

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

    /// Same as [`std::vec::Vec::truncate`].
    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len);
    }

    /// Same as [`std::vec::Vec::resize`] but returns an error on allocation
    /// failure.
    pub fn resize(&mut self, new_len: usize, value: T) -> Result<(), OutOfMemory>
    where
        T: TryClone,
    {
        match new_len.cmp(&self.len()) {
            Ordering::Less => self.truncate(new_len),
            Ordering::Equal => {}
            Ordering::Greater => {
                let delta = new_len - self.len();
                self.reserve(delta)?;
                // Minimize `try_clone` calls by always pushing `value` directly
                // as the last element.
                for _ in 0..delta - 1 {
                    self.push(value.try_clone()?)?;
                }
                self.push(value)?;
            }
        }
        Ok(())
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

    /// Same as [`std::vec::Vec::shrink_to_fit`] but returns an error on
    /// allocation failure.
    pub fn shrink_to_fit(&mut self) -> Result<(), OutOfMemory> {
        // If our length is already equal to our capacity, then there is nothing
        // to shrink.
        if self.len() == self.capacity() {
            return Ok(());
        }

        // `realloc` requires a non-zero original layout as well as a non-zero
        // destination layout, so this guard ensures that the sizes below are
        // all nonzero. This handles a few cases:
        //
        // * If `len == cap == 0` then no allocation has ever been made.
        // * If `len == 0` and `cap != 0` then this function effectively frees
        //   the memory.
        // * If `T` is a zero-sized type then nothing's been allocated either.
        //
        // In all of these cases delegate to the standard library's
        // `shrink_to_fit` which is guaranteed to not perform a `realloc`.
        if self.is_empty() || mem::size_of::<T>() == 0 {
            self.inner.shrink_to_fit();
            return Ok(());
        }

        let (ptr, len, cap) = mem::take(self).into_raw_parts();
        let layout = Layout::array::<T>(cap).unwrap();
        let new_size = Layout::array::<T>(len).unwrap().size();

        // SAFETY: `ptr` was previously allocated in the global allocator,
        // `layout` has a nonzero size and matches the current allocation of
        // `ptr`, `new_size` is nonzero, and `new_size` is a valid array size
        // for `len` elements given its constructor.
        let result = unsafe { try_realloc(ptr.cast(), layout, new_size) };

        match result {
            Ok(ptr) => {
                // SAFETY: `result` is allocated with the global allocator and
                // has room for exactly `[T; len]`.
                *self = unsafe { Self::from_raw_parts(ptr.cast::<T>().as_ptr(), len, len) };
                Ok(())
            }
            Err(oom) => {
                // SAFETY: If reallocation fails then it's guaranteed that the
                // original allocation is not tampered with, so it's safe to
                // reassemble the original vector.
                *self = unsafe { Vec::from_raw_parts(ptr, len, cap) };
                Err(oom)
            }
        }
    }

    /// Same as [`std::vec::Vec::into_boxed_slice`] but returns an error on
    /// allocation failure.
    pub fn into_boxed_slice(mut self) -> Result<Box<[T]>, OutOfMemory> {
        self.shrink_to_fit()?;

        // Once we've shrunken the allocation to just the actual length, we can
        // use `std`'s `into_boxed_slice` without fear of `realloc`.
        Ok(self.inner.into_boxed_slice())
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

impl<T, I> Index<I> for Vec<T>
where
    I: SliceIndex<[T]>,
{
    type Output = <I as SliceIndex<[T]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T, I> IndexMut<I> for Vec<T>
where
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
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

impl<T> From<Vec<T>> for StdVec<T> {
    fn from(v: Vec<T>) -> Self {
        v.inner
    }
}

impl<T> From<StdVec<T>> for Vec<T> {
    fn from(inner: StdVec<T>) -> Self {
        Self { inner }
    }
}

impl<T> From<Box<[T]>> for Vec<T> {
    fn from(boxed_slice: Box<[T]>) -> Self {
        Self::from(StdVec::from(boxed_slice))
    }
}

impl<T> serde::ser::Serialize for Vec<T>
where
    T: serde::ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for elem in self {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }
}

impl<'de, T> serde::de::Deserialize<'de> for Vec<T>
where
    T: serde::de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<fn() -> Vec<T>>);

        impl<'de, T> serde::de::Visitor<'de> for Visitor<T>
        where
            T: serde::de::Deserialize<'de>,
        {
            type Value = Vec<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a `wasmtime_core::alloc::Vec` sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error as _;

                let mut v = Vec::new();

                if let Some(len) = seq.size_hint() {
                    v.reserve_exact(len).map_err(|oom| A::Error::custom(oom))?;
                }

                while let Some(elem) = seq.next_element()? {
                    v.push(elem).map_err(|oom| A::Error::custom(oom))?;
                }

                Ok(v)
            }
        }

        deserializer.deserialize_seq(Visitor(PhantomData))
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
