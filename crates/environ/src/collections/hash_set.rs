use crate::error::OutOfMemory;
use core::{
    fmt,
    hash::{BuildHasher, Hash},
};
use hashbrown::{DefaultHashBuilder, Equivalent, hash_set as inner};

/// A wrapper type around [`hashbrown::hash_set::HashSet`] that only exposes
/// fallible allocation.
pub struct HashSet<T, S = DefaultHashBuilder> {
    inner: inner::HashSet<T, S>,
}

impl<T, S> PartialEq for HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T, S> Eq for HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
}

impl<T, S> fmt::Debug for HashSet<T, S>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T> HashSet<T, DefaultHashBuilder> {
    /// Same as [`hashbrown::hash_set::HashSet::new`].
    pub fn new() -> Self {
        Self {
            inner: inner::HashSet::new(),
        }
    }
}

impl<T> HashSet<T, DefaultHashBuilder>
where
    T: Eq + Hash,
{
    /// Same as [`hashbrown::hash_set::HashSet::with_capacity`] but returns an
    /// error on allocation failure.
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut set = Self::new();
        set.reserve(capacity)?;
        Ok(set)
    }
}

impl<T, S> HashSet<T, S> {
    /// Same as [`hashbrown::hash_set::HashSet::with_hasher`].
    pub const fn with_hasher(hasher: S) -> Self {
        Self {
            inner: inner::HashSet::with_hasher(hasher),
        }
    }

    /// Same as [`hashbrown::hash_set::HashSet::capacity`].
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Same as [`hashbrown::hash_set::HashSet::iter`].
    pub fn iter(&self) -> inner::Iter<'_, T> {
        self.inner.iter()
    }

    /// Same as [`hashbrown::hash_set::HashSet::len`].
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Same as [`hashbrown::hash_set::HashSet::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`hashbrown::hash_set::HashSet::drain`].
    pub fn drain(&mut self) -> inner::Drain<'_, T> {
        self.inner.drain()
    }

    /// Same as [`hashbrown::hash_set::HashSet::retain`].
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.retain(f);
    }

    /// Same as [`hashbrown::hash_set::HashSet::extract_if`].
    pub fn extract_if<F>(&mut self, f: F) -> inner::ExtractIf<'_, T, F>
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.extract_if(f)
    }

    /// Same as [`hashbrown::hash_set::HashSet::clear`].
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<T, S> HashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    /// Same as [`hashbrown::hash_set::HashSet::with_capacity_and_hasher`] but
    /// returns an error on allocation failure.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Result<Self, OutOfMemory> {
        let mut map = Self::with_hasher(hasher);
        map.reserve(capacity)?;
        Ok(map)
    }

    /// Same as [`hashbrown::hash_set::HashSet::reserve`] but returns an error
    /// on allocation failure.
    pub fn reserve(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner
            .try_reserve(additional)
            .map_err(|_| OutOfMemory::new(self.len().saturating_add(additional)))
    }

    /// Same as [`hashbrown::hash_set::HashSet::contains`].
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.inner.contains(value)
    }

    /// Same as [`hashbrown::hash_set::HashSet::get`].
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.inner.get(value)
    }

    /// Same as [`hashbrown::hash_set::HashSet::entry`] but pre-reserves space
    /// for the value if it is not contained and returns an error on allocation
    /// failure.
    pub fn entry(&mut self, value: T) -> Result<inner::Entry<'_, T, S>, OutOfMemory> {
        if !self.contains(&value) {
            self.reserve(1)?;
        }
        Ok(self.inner.entry(value))
    }

    /// Same as [`hashbrown::hash_set::HashSet::insert`] but returns an error on
    /// allocation failure.
    pub fn insert(&mut self, value: T) -> Result<bool, OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.insert(value))
    }

    /// Same as [`hashbrown::hash_set::HashSet::remove`].
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.inner.remove(value)
    }

    /// Same as [`hashbrown::hash_set::HashSet::take`].
    pub fn take<Q>(&mut self, value: &Q) -> Option<T>
    where
        Q: Hash + Equivalent<T> + ?Sized,
    {
        self.inner.take(value)
    }
}
