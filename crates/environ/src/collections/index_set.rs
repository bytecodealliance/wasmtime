use crate::{
    collections::{TryClone, TryFromIterator},
    error::OutOfMemory,
};
use core::{
    borrow::Borrow,
    fmt,
    hash::{BuildHasher, Hash},
    marker::PhantomData,
    mem,
    ops::Index,
};
use indexmap::set as inner;

#[cfg(feature = "std")]
use std::hash::RandomState as DefaultHashBuilder;

#[cfg(not(feature = "std"))]
use hashbrown::DefaultHashBuilder;

/// A wrapper around [`indexmap::IndexSet`] that only provides fallible
/// allocation.
pub struct TryIndexSet<T, S = DefaultHashBuilder> {
    inner: indexmap::IndexSet<T, S>,
}

impl<T, S> Default for TryIndexSet<T, S>
where
    S: Default,
{
    fn default() -> Self {
        Self {
            inner: indexmap::IndexSet::default(),
        }
    }
}

impl<T, S> PartialEq for TryIndexSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T, S> Eq for TryIndexSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
}

impl<T, S> TryClone for TryIndexSet<T, S>
where
    T: Eq + Hash + TryClone,
    S: BuildHasher + TryClone,
{
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        let mut set = Self::with_hasher(self.inner.hasher().try_clone()?);
        set.reserve(self.len())?;
        for value in self.iter() {
            set.insert(value.try_clone()?)?;
        }
        Ok(set)
    }
}

impl<T, S> fmt::Debug for TryIndexSet<T, S>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T, S> From<TryIndexSet<T, S>> for indexmap::IndexSet<T, S> {
    fn from(set: TryIndexSet<T, S>) -> Self {
        set.inner
    }
}

impl<T, S> From<indexmap::IndexSet<T, S>> for TryIndexSet<T, S> {
    fn from(inner: indexmap::IndexSet<T, S>) -> Self {
        Self { inner }
    }
}

impl<T, S> serde::ser::Serialize for TryIndexSet<T, S>
where
    T: serde::ser::Serialize,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        use serde::ser::SerializeSeq as _;
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for value in self {
            seq.serialize_element(value)?;
        }
        seq.end()
    }
}

impl<'de, T> serde::de::Deserialize<'de> for TryIndexSet<T>
where
    T: serde::de::Deserialize<'de> + Hash + Eq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<fn() -> TryIndexSet<T>>);

        impl<'de, T> serde::de::Visitor<'de> for Visitor<T>
        where
            T: serde::de::Deserialize<'de> + Hash + Eq,
        {
            type Value = TryIndexSet<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an `IndexSet`")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                use serde::de::Error as _;

                let mut result = TryIndexSet::<T>::new();

                if let Some(len) = seq.size_hint() {
                    result.reserve(len).map_err(|oom| A::Error::custom(oom))?;
                }

                while let Some(value) = seq.next_element::<T>()? {
                    result.insert(value).map_err(|oom| A::Error::custom(oom))?;
                }

                Ok(result)
            }
        }

        deserializer.deserialize_seq(Visitor(PhantomData))
    }
}

impl<T> TryFromIterator<T, OutOfMemory> for TryIndexSet<T>
where
    T: Hash + Eq,
{
    fn try_from_iter<I>(iter: I) -> Result<Self, OutOfMemory>
    where
        I: Iterator<Item = T>,
    {
        let mut set = Self::new();
        set.reserve(iter.size_hint().0)?;
        for value in iter {
            set.insert(value)?;
        }
        Ok(set)
    }
}

impl<T, E> TryFromIterator<Result<T, E>, E> for TryIndexSet<T>
where
    T: Hash + Eq,
    E: From<OutOfMemory>,
{
    fn try_from_iter<I>(iter: I) -> Result<Self, E>
    where
        I: Iterator<Item = Result<T, E>>,
    {
        let mut set = Self::new();
        set.reserve(iter.size_hint().0)?;
        for value in iter {
            set.insert(value?)?;
        }
        Ok(set)
    }
}

impl<T> TryIndexSet<T> {
    /// Same as [`indexmap::IndexSet::new`].
    pub fn new() -> Self {
        Self {
            inner: indexmap::IndexSet::with_hasher(Default::default()),
        }
    }
}

impl<T, S> TryIndexSet<T, S> {
    /// Same as [`indexmap::IndexSet::with_hasher`].
    pub const fn with_hasher(hash_builder: S) -> Self {
        TryIndexSet {
            inner: indexmap::IndexSet::with_hasher(hash_builder),
        }
    }

    /// Same as [`indexmap::IndexSet::len`].
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Same as [`indexmap::IndexSet::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`indexmap::IndexSet::iter`].
    pub fn iter(&self) -> inner::Iter<'_, T> {
        self.inner.iter()
    }

    /// Same as [`indexmap::IndexSet::get_index`].
    pub fn get_index(&self, index: usize) -> Option<&T> {
        self.inner.get_index(index)
    }

    /// Same as [`indexmap::IndexSet::reserve`] but returns an error on
    /// allocation failure.
    pub fn reserve(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve(additional).map_err(|_| {
            let new_len = self.len().saturating_add(additional);
            OutOfMemory::new(new_len.saturating_mul(mem::size_of::<T>()))
        })
    }
}

impl<T, S> TryIndexSet<T, S>
where
    T: Hash + Eq,
    S: BuildHasher,
{
    /// Same as [`indexmap::IndexSet::insert`] but returns an error on
    /// allocation failure.
    pub fn insert(&mut self, value: T) -> Result<bool, OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.insert(value))
    }

    /// Same as [`indexmap::IndexSet::contains`].
    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq,
        T: Borrow<Q>,
    {
        self.inner.contains(value)
    }

    /// Same as [`indexmap::IndexSet::get_index_of`].
    pub fn get_index_of<Q>(&self, value: &Q) -> Option<usize>
    where
        Q: ?Sized + Hash + Eq,
        T: Borrow<Q>,
    {
        self.inner.get_index_of(value)
    }
}

impl<T, S> Index<usize> for TryIndexSet<T, S> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.inner[index]
    }
}

impl<'a, T, S> IntoIterator for &'a TryIndexSet<T, S> {
    type Item = &'a T;
    type IntoIter = inner::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, S> IntoIterator for TryIndexSet<T, S> {
    type Item = T;
    type IntoIter = inner::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
