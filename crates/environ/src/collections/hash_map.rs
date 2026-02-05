use crate::error::OutOfMemory;
use core::{
    borrow::Borrow,
    fmt,
    hash::{BuildHasher, Hash},
    mem,
};

#[cfg(feature = "std")]
use std::{collections::hash_map as inner, hash::RandomState as DefaultHashBuilder};

#[cfg(not(feature = "std"))]
use hashbrown::{DefaultHashBuilder, hash_map as inner};
use wasmtime_core::alloc::TryClone;

/// A wrapper type around [`hashbrown::hash_map::HashMap`] that only exposes
/// fallible allocation.
pub struct HashMap<K, V, S = DefaultHashBuilder> {
    inner: inner::HashMap<K, V, S>,
}

impl<K, V, S> TryClone for HashMap<K, V, S>
where
    K: Eq + Hash + TryClone,
    V: TryClone,
    S: BuildHasher + TryClone,
{
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        let mut map = Self::with_capacity_and_hasher(self.len(), self.hasher().try_clone()?)?;
        for (k, v) in self {
            map.insert(k.try_clone()?, v.try_clone()?)
                .expect("reserved capacity");
        }
        Ok(map)
    }
}

impl<K, V, S> Default for HashMap<K, V, S>
where
    S: Default,
{
    fn default() -> Self {
        Self {
            inner: inner::HashMap::default(),
        }
    }
}

impl<K, V, S> PartialEq for HashMap<K, V, S>
where
    K: Eq + Hash,
    V: PartialEq,
    S: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<K, V, S> Eq for HashMap<K, V, S>
where
    K: Eq + Hash,
    V: Eq,
    S: BuildHasher,
{
}

impl<K, V, S> fmt::Debug for HashMap<K, V, S>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<K, V> HashMap<K, V, DefaultHashBuilder> {
    /// Same as [`hashbrown::hash_map::HashMap::new`].
    pub fn new() -> Self {
        Self {
            inner: inner::HashMap::new(),
        }
    }
}

impl<K, V> HashMap<K, V, DefaultHashBuilder>
where
    K: Eq + Hash,
{
    /// Same as [`hashbrown::hash_map::HashMap::with_capacity`] but returns an
    /// error on allocation failure.
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut map = Self::new();
        map.reserve(capacity)?;
        Ok(map)
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Same as [`hashbrown::hash_map::HashMap::with_hasher`].
    pub const fn with_hasher(hasher: S) -> Self {
        Self {
            inner: inner::HashMap::with_hasher(hasher),
        }
    }

    /// Same as [`hashbrown::hash_map::HashMap::hasher`].
    pub fn hasher(&self) -> &S {
        self.inner.hasher()
    }

    /// Same as [`hashbrown::hash_map::HashMap::capacity`].
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Same as [`hashbrown::hash_map::HashMap::len`].
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Same as [`hashbrown::hash_map::HashMap::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`hashbrown::hash_map::HashMap::drain`].
    pub fn drain(&mut self) -> inner::Drain<'_, K, V> {
        self.inner.drain()
    }

    /// Same as [`hashbrown::hash_map::HashMap::retain`].
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.inner.retain(f);
    }

    /// Same as [`hashbrown::hash_map::HashMap::extract_if`].
    pub fn extract_if<F>(&mut self, f: F) -> inner::ExtractIf<'_, K, V, F>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.inner.extract_if(f)
    }

    /// Same as [`hashbrown::hash_map::HashMap::clear`].
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Same as [`hashbrown::hash_map::HashMap::iter`].
    pub fn iter(&self) -> inner::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Same as [`hashbrown::hash_map::HashMap::iter_mut`].
    pub fn iter_mut(&mut self) -> inner::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Same as [`hashbrown::hash_map::HashMap::with_capacity_and_hasher`] but
    /// returns an error on allocation failure.
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Result<Self, OutOfMemory> {
        let mut map = Self::with_hasher(hasher);
        map.reserve(capacity)?;
        Ok(map)
    }

    /// Same as [`hashbrown::hash_map::HashMap::reserve`] but returns an error
    /// on allocation failure.
    pub fn reserve(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve(additional).map_err(|_| {
            let new_len = self.len().saturating_add(additional);
            OutOfMemory::new(
                new_len
                    .saturating_mul(mem::size_of::<K>())
                    .saturating_add(new_len.saturating_mul(mem::size_of::<V>())),
            )
        })
    }

    /// Same as [`hashbrown::hash_map::HashMap::contains`].
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: Hash + Eq + ?Sized,
        K: Borrow<Q>,
    {
        self.inner.contains_key(key)
    }

    /// Same as [`hashbrown::hash_map::HashMap::get`].
    pub fn get<Q>(&self, value: &Q) -> Option<&V>
    where
        Q: Hash + Eq + ?Sized,
        K: Borrow<Q>,
    {
        self.inner.get(value)
    }

    /// Same as [`hashbrown::hash_map::HashMap::insert`] but returns an error on
    /// allocation failure.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.insert(key, value))
    }

    /// Same as [`hashbrown::hash_map::HashMap::remove`].
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: Hash + Eq + ?Sized,
        K: Borrow<Q>,
    {
        self.inner.remove(key)
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);

    type IntoIter = inner::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);

    type IntoIter = inner::IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);

    type IntoIter = inner::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
