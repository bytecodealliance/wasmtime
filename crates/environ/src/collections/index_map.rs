use crate::{collections::TryClone, error::OutOfMemory};
use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    hash::{BuildHasher, Hash},
    marker::PhantomData,
    mem,
    ops::{Index, IndexMut, RangeBounds},
};
use indexmap::map as inner;

#[cfg(feature = "std")]
use std::hash::RandomState as DefaultHashBuilder;

#[cfg(not(feature = "std"))]
use hashbrown::DefaultHashBuilder;

/// A wrapper around [`indexmap::IndexMap`] that only provides fallible
/// allocation.
pub struct IndexMap<K, V, S = DefaultHashBuilder> {
    inner: indexmap::IndexMap<K, V, S>,
}

impl<K, V, S> TryClone for IndexMap<K, V, S>
where
    K: Eq + Hash + TryClone,
    V: TryClone,
    S: BuildHasher + TryClone,
{
    fn try_clone(&self) -> Result<Self, OutOfMemory> {
        let mut map = Self::with_capacity_and_hasher(self.capacity(), self.hasher().try_clone()?)?;
        for (k, v) in self.iter() {
            map.insert(k.try_clone()?, v.try_clone()?)?;
        }
        Ok(map)
    }
}

impl<K, V, S> fmt::Debug for IndexMap<K, V, S>
where
    K: fmt::Debug,
    V: fmt::Debug,
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<K, V, S> From<IndexMap<K, V, S>> for indexmap::IndexMap<K, V, S> {
    fn from(map: IndexMap<K, V, S>) -> Self {
        map.inner
    }
}

impl<K, V, S> From<indexmap::IndexMap<K, V, S>> for IndexMap<K, V, S> {
    fn from(inner: indexmap::IndexMap<K, V, S>) -> Self {
        Self { inner }
    }
}

impl<K, V, H> serde::ser::Serialize for IndexMap<K, V, H>
where
    K: serde::ser::Serialize,
    V: serde::ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap as _;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de, K, V> serde::de::Deserialize<'de> for IndexMap<K, V>
where
    K: serde::de::Deserialize<'de> + Hash + Eq,
    V: serde::de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<K, V>(PhantomData<fn() -> IndexMap<K, V>>);

        impl<'de, K, V> serde::de::Visitor<'de> for Visitor<K, V>
        where
            K: serde::de::Deserialize<'de> + Hash + Eq,
            V: serde::de::Deserialize<'de>,
        {
            type Value = IndexMap<K, V>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an `IndexMap`")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                use serde::de::Error as _;

                let mut result = IndexMap::<K, V>::new();

                if let Some(len) = map.size_hint() {
                    result.reserve(len).map_err(|oom| A::Error::custom(oom))?;
                }

                while let Some((k, v)) = map.next_entry::<K, V>()? {
                    result.insert(k, v).map_err(|oom| A::Error::custom(oom))?;
                }

                Ok(result)
            }
        }

        deserializer.deserialize_map(Visitor(PhantomData))
    }
}

impl<K, V> IndexMap<K, V> {
    /// Same as [`indexmap::IndexMap::new`].
    pub fn new() -> Self {
        Self {
            inner: indexmap::IndexMap::with_hasher(<_>::default()),
        }
    }

    /// Same as [`indexmap::IndexMap::with_capacity`] but returns an error on
    /// allocation failure.
    pub fn with_capacity(n: usize) -> Result<Self, OutOfMemory> {
        Self::with_capacity_and_hasher(n, <_>::default())
    }
}

impl<K, V, S> IndexMap<K, V, S> {
    /// Same as [`indexmap::IndexMap::with_capacity_and_hasher`] but returns an
    /// error on allocation failure.
    pub fn with_capacity_and_hasher(n: usize, hash_builder: S) -> Result<Self, OutOfMemory> {
        let mut map = Self::with_hasher(hash_builder);
        map.reserve(n)?;
        Ok(map)
    }

    /// Same as [`indexmap::IndexMap::with_hasher`].
    pub const fn with_hasher(hash_builder: S) -> Self {
        IndexMap {
            inner: indexmap::IndexMap::with_hasher(hash_builder),
        }
    }

    /// Same as [`indexmap::IndexMap::capacity`].
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Same as [`indexmap::IndexMap::hasher`].
    pub fn hasher(&self) -> &S {
        self.inner.hasher()
    }

    /// Same as [`indexmap::IndexMap::len`].
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Same as [`indexmap::IndexMap::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`indexmap::IndexMap::iter`].
    pub fn iter(&self) -> inner::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Same as [`indexmap::IndexMap::iter_mut`].
    pub fn iter_mut(&mut self) -> inner::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }

    /// Same as [`indexmap::IndexMap::keys`].
    pub fn keys(&self) -> inner::Keys<'_, K, V> {
        self.inner.keys()
    }

    /// Same as [`indexmap::IndexMap::into_keys`].
    pub fn into_keys(self) -> inner::IntoKeys<K, V> {
        self.inner.into_keys()
    }

    /// Same as [`indexmap::IndexMap::values`].
    pub fn values(&self) -> inner::Values<'_, K, V> {
        self.inner.values()
    }

    /// Same as [`indexmap::IndexMap::values_mut`].
    pub fn values_mut(&mut self) -> inner::ValuesMut<'_, K, V> {
        self.inner.values_mut()
    }

    /// Same as [`indexmap::IndexMap::into_values`].
    pub fn into_values(self) -> inner::IntoValues<K, V> {
        self.inner.into_values()
    }

    /// Same as [`indexmap::IndexMap::clear`].
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Same as [`indexmap::IndexMap::truncate`].
    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len);
    }

    /// Same as [`indexmap::IndexMap::drain`].
    pub fn drain<R>(&mut self, range: R) -> inner::Drain<'_, K, V>
    where
        R: RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    /// Same as [`indexmap::IndexMap::extract_if`].
    pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> inner::ExtractIf<'_, K, V, F>
    where
        F: FnMut(&K, &mut V) -> bool,
        R: RangeBounds<usize>,
    {
        self.inner.extract_if(range, pred)
    }

    /// Same as [`indexmap::IndexMap::split_off`] but returns an error on
    /// allocation failure.
    pub fn split_off(&mut self, at: usize) -> Result<Self, OutOfMemory>
    where
        K: Eq + Hash,
        S: BuildHasher + TryClone,
    {
        assert!(at <= self.len());
        let mut map = Self::with_capacity_and_hasher(self.len() - at, self.hasher().try_clone()?)?;
        for (k, v) in self.drain(at..) {
            map.insert(k, v)?;
        }
        Ok(map)
    }

    /// Same as [`indexmap::IndexMap::reserve`] but returns an error on
    /// allocation failure.
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

    /// Same as [`indexmap::IndexMap::reserve_exact`] but returns an error on
    /// allocation failure.
    pub fn reserve_exact(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve_exact(additional).map_err(|_| {
            let new_len = self.len().saturating_add(additional);
            OutOfMemory::new(
                new_len
                    .saturating_mul(mem::size_of::<K>())
                    .saturating_add(new_len.saturating_mul(mem::size_of::<V>())),
            )
        })
    }
}

impl<K, V, S> IndexMap<K, V, S>
where
    K: Hash + Eq,
    S: BuildHasher,
{
    /// Same as [`indexmap::IndexMap::insert`] but returns an error on
    /// allocation failure.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.insert(key, value))
    }

    /// Same as [`indexmap::IndexMap::insert_full`] but returns an error on
    /// allocation failure.
    pub fn insert_full(&mut self, key: K, value: V) -> Result<(usize, Option<V>), OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.insert_full(key, value))
    }

    /// Same as [`indexmap::IndexMap::insert_sorted`] but returns an error on
    /// allocation failure.
    pub fn insert_sorted(&mut self, key: K, value: V) -> Result<(usize, Option<V>), OutOfMemory>
    where
        K: Ord,
    {
        self.reserve(1)?;
        Ok(self.inner.insert_sorted(key, value))
    }

    /// Same as [`indexmap::IndexMap::insert_sorted_by`] but returns an error on
    /// allocation failure.
    pub fn insert_sorted_by<F>(
        &mut self,
        key: K,
        value: V,
        cmp: F,
    ) -> Result<(usize, Option<V>), OutOfMemory>
    where
        F: FnMut(&K, &V, &K, &V) -> Ordering,
    {
        self.reserve(1)?;
        Ok(self.inner.insert_sorted_by(key, value, cmp))
    }

    /// Same as [`indexmap::IndexMap::insert_sorted_by_key`] but returns an
    /// error on allocation failure.
    pub fn insert_sorted_by_key<B, F>(
        &mut self,
        key: K,
        value: V,
        sort_key: F,
    ) -> Result<(usize, Option<V>), OutOfMemory>
    where
        B: Ord,
        F: FnMut(&K, &V) -> B,
    {
        self.reserve(1)?;
        Ok(self.inner.insert_sorted_by_key(key, value, sort_key))
    }

    /// Same as [`indexmap::IndexMap::insert_before`] but returns an error on
    /// allocation failure.
    pub fn insert_before(
        &mut self,
        index: usize,
        key: K,
        value: V,
    ) -> Result<(usize, Option<V>), OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.insert_before(index, key, value))
    }

    /// Same as [`indexmap::IndexMap::shift_insert`] but returns an error on
    /// allocation failure.
    pub fn shift_insert(
        &mut self,
        index: usize,
        key: K,
        value: V,
    ) -> Result<Option<V>, OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.shift_insert(index, key, value))
    }

    /// Same as [`indexmap::IndexMap::shift_insert`].
    pub fn replace_index(&mut self, index: usize, key: K) -> Result<K, (usize, K)> {
        self.inner.replace_index(index, key)
    }
}

impl<K, V, S> IndexMap<K, V, S>
where
    S: BuildHasher,
{
    /// Same as [`indexmap::IndexMap::contains_key`].
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.contains_key(key)
    }

    /// Same as [`indexmap::IndexMap::get`].
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get(key)
    }

    /// Same as [`indexmap::IndexMap::get_key_value`].
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get_key_value(key)
    }

    /// Same as [`indexmap::IndexMap::get_full`].
    pub fn get_full<Q>(&self, key: &Q) -> Option<(usize, &K, &V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get_full(key)
    }

    /// Same as [`indexmap::IndexMap::get_index_of`].
    pub fn get_index_of<Q>(&self, key: &Q) -> Option<usize>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get_index_of(key)
    }

    /// Same as [`indexmap::IndexMap::get_mut`].
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get_mut(key)
    }

    /// Same as [`indexmap::IndexMap::get_key_value_mut`].
    pub fn get_key_value_mut<Q>(&mut self, key: &Q) -> Option<(&K, &mut V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get_key_value_mut(key)
    }

    /// Same as [`indexmap::IndexMap::get_full_mut`].
    pub fn get_full_mut<Q>(&mut self, key: &Q) -> Option<(usize, &K, &mut V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get_full_mut(key)
    }

    /// Same as [`indexmap::IndexMap::get_disjoint_mut`].
    pub fn get_disjoint_mut<Q, const N: usize>(&mut self, keys: [&Q; N]) -> [Option<&mut V>; N]
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.get_disjoint_mut(keys)
    }

    /// Same as [`indexmap::IndexMap::swap_remove`].
    pub fn swap_remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.swap_remove(key)
    }

    /// Same as [`indexmap::IndexMap::swap_remove_entry`].
    pub fn swap_remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.swap_remove_entry(key)
    }

    /// Same as [`indexmap::IndexMap::swap_remove_full`].
    pub fn swap_remove_full<Q>(&mut self, key: &Q) -> Option<(usize, K, V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.swap_remove_full(key)
    }

    /// Same as [`indexmap::IndexMap::shift_remove`].
    pub fn shift_remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.shift_remove(key)
    }

    /// Same as [`indexmap::IndexMap::shift_remove_entry`].
    pub fn shift_remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.shift_remove_entry(key)
    }

    /// Same as [`indexmap::IndexMap::shift_remove_full`].
    pub fn shift_remove_full<Q>(&mut self, key: &Q) -> Option<(usize, K, V)>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q>,
    {
        self.inner.shift_remove_full(key)
    }
}

impl<K, V, S> IndexMap<K, V, S> {
    /// Same as [`indexmap::IndexMap::pop`].
    pub fn pop(&mut self) -> Option<(K, V)> {
        self.inner.pop()
    }

    /// Same as [`indexmap::IndexMap::retain`].
    pub fn retain<F>(&mut self, keep: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.inner.retain(keep);
    }

    /// Same as [`indexmap::IndexMap::sort_keys`].
    pub fn sort_keys(&mut self)
    where
        K: Ord,
    {
        self.inner.sort_keys();
    }

    /// Same as [`indexmap::IndexMap::sort_by`].
    pub fn sort_by<F>(&mut self, cmp: F)
    where
        F: FnMut(&K, &V, &K, &V) -> Ordering,
    {
        self.inner.sort_by(cmp);
    }

    /// Same as [`indexmap::IndexMap::sorted_by`].
    pub fn sorted_by<F>(self, cmp: F) -> inner::IntoIter<K, V>
    where
        F: FnMut(&K, &V, &K, &V) -> Ordering,
    {
        self.inner.sorted_by(cmp)
    }

    /// Same as [`indexmap::IndexMap::sort_by_key`].
    pub fn sort_by_key<T, F>(&mut self, sort_key: F)
    where
        T: Ord,
        F: FnMut(&K, &V) -> T,
    {
        self.inner.sort_by_key(sort_key);
    }

    /// Same as [`indexmap::IndexMap::sort_unstable_keys`].
    pub fn sort_unstable_keys(&mut self)
    where
        K: Ord,
    {
        self.inner.sort_unstable_keys();
    }

    /// Same as [`indexmap::IndexMap::sort_unstable_by`].
    pub fn sort_unstable_by<F>(&mut self, cmp: F)
    where
        F: FnMut(&K, &V, &K, &V) -> Ordering,
    {
        self.inner.sort_unstable_by(cmp);
    }

    /// Same as [`indexmap::IndexMap::sorted_unstable_by`].
    pub fn sorted_unstable_by<F>(self, cmp: F) -> inner::IntoIter<K, V>
    where
        F: FnMut(&K, &V, &K, &V) -> Ordering,
    {
        self.inner.sorted_unstable_by(cmp)
    }

    /// Same as [`indexmap::IndexMap::sort_unstable_by_key`].
    pub fn sort_unstable_by_key<T, F>(&mut self, sort_key: F)
    where
        T: Ord,
        F: FnMut(&K, &V) -> T,
    {
        self.inner.sort_unstable_by_key(sort_key);
    }

    /// Same as [`indexmap::IndexMap::binary_search_keys`].
    pub fn binary_search_keys(&self, x: &K) -> Result<usize, usize>
    where
        K: Ord,
    {
        self.inner.binary_search_keys(x)
    }

    /// Same as [`indexmap::IndexMap::binary_search_by`].
    pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a K, &'a V) -> Ordering,
    {
        self.inner.binary_search_by(f)
    }

    /// Same as [`indexmap::IndexMap::binary_search_by_key`].
    pub fn binary_search_by_key<'a, B, F>(&'a self, b: &B, f: F) -> Result<usize, usize>
    where
        F: FnMut(&'a K, &'a V) -> B,
        B: Ord,
    {
        self.inner.binary_search_by_key(b, f)
    }

    /// Same as [`indexmap::IndexMap::is_sorted`].
    pub fn is_sorted(&self) -> bool
    where
        K: PartialOrd,
    {
        self.inner.is_sorted()
    }

    /// Same as [`indexmap::IndexMap::is_sorted_by`].
    pub fn is_sorted_by<'a, F>(&'a self, cmp: F) -> bool
    where
        F: FnMut(&'a K, &'a V, &'a K, &'a V) -> bool,
    {
        self.inner.is_sorted_by(cmp)
    }

    /// Same as [`indexmap::IndexMap::is_sorted_by_key`].
    pub fn is_sorted_by_key<'a, F, T>(&'a self, sort_key: F) -> bool
    where
        F: FnMut(&'a K, &'a V) -> T,
        T: PartialOrd,
    {
        self.inner.is_sorted_by_key(sort_key)
    }

    /// Same as [`indexmap::IndexMap::partition_point`].
    #[must_use]
    pub fn partition_point<P>(&self, pred: P) -> usize
    where
        P: FnMut(&K, &V) -> bool,
    {
        self.inner.partition_point(pred)
    }

    /// Same as [`indexmap::IndexMap::reverse`].
    pub fn reverse(&mut self) {
        self.inner.reverse()
    }

    /// Same as [`indexmap::IndexMap::get_index`].
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        self.inner.get_index(index)
    }

    /// Same as [`indexmap::IndexMap::get_index_mut`].
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&K, &mut V)> {
        self.inner.get_index_mut(index)
    }

    /// Same as [`indexmap::IndexMap::get_disjoint_indices_mut`].
    pub fn get_disjoint_indices_mut<const N: usize>(
        &mut self,
        indices: [usize; N],
    ) -> Result<[(&K, &mut V); N], indexmap::GetDisjointMutError> {
        self.inner.get_disjoint_indices_mut(indices)
    }

    /// Same as [`indexmap::IndexMap::first`].
    pub fn first(&self) -> Option<(&K, &V)> {
        self.inner.first()
    }

    /// Same as [`indexmap::IndexMap::first_mut`].
    pub fn first_mut(&mut self) -> Option<(&K, &mut V)> {
        self.inner.first_mut()
    }

    /// Same as [`indexmap::IndexMap::last`].
    pub fn last(&self) -> Option<(&K, &V)> {
        self.inner.last()
    }

    /// Same as [`indexmap::IndexMap::last_mut`].
    pub fn last_mut(&mut self) -> Option<(&K, &mut V)> {
        self.inner.last_mut()
    }

    /// Same as [`indexmap::IndexMap::swap_remove_index`].
    pub fn swap_remove_index(&mut self, index: usize) -> Option<(K, V)> {
        self.inner.swap_remove_index(index)
    }

    /// Same as [`indexmap::IndexMap::shift_remove_index`].
    pub fn shift_remove_index(&mut self, index: usize) -> Option<(K, V)> {
        self.inner.shift_remove_index(index)
    }

    /// Same as [`indexmap::IndexMap::move_index`].
    pub fn move_index(&mut self, from: usize, to: usize) {
        self.inner.move_index(from, to)
    }

    /// Same as [`indexmap::IndexMap::swap_indices`].
    pub fn swap_indices(&mut self, a: usize, b: usize) {
        self.inner.swap_indices(a, b)
    }
}

impl<K, V, Q, S> Index<&Q> for IndexMap<K, V, S>
where
    Q: ?Sized + Hash + Eq,
    K: Borrow<Q>,
    S: BuildHasher,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        &self.inner[key]
    }
}

impl<K, V, Q, S> IndexMut<&Q> for IndexMap<K, V, S>
where
    Q: ?Sized + Hash + Eq,
    K: Borrow<Q>,
    S: BuildHasher,
{
    fn index_mut(&mut self, key: &Q) -> &mut V {
        &mut self.inner[key]
    }
}

impl<K, V, S> Index<usize> for IndexMap<K, V, S> {
    type Output = V;

    fn index(&self, index: usize) -> &V {
        &self.inner[index]
    }
}

impl<K, V, S> IndexMut<usize> for IndexMap<K, V, S> {
    fn index_mut(&mut self, index: usize) -> &mut V {
        &mut self.inner[index]
    }
}

impl<K, V, S> Default for IndexMap<K, V, S>
where
    S: Default,
{
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K, V1, S1, V2, S2> PartialEq<IndexMap<K, V2, S2>> for IndexMap<K, V1, S1>
where
    K: Hash + Eq,
    V1: PartialEq<V2>,
    S1: BuildHasher,
    S2: BuildHasher,
{
    fn eq(&self, other: &IndexMap<K, V2, S2>) -> bool {
        &self.inner == &other.inner
    }
}

impl<K, V, S> Eq for IndexMap<K, V, S>
where
    K: Eq + Hash,
    V: Eq,
    S: BuildHasher,
{
}

impl<'a, K, V, S> IntoIterator for &'a IndexMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = inner::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut IndexMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = inner::IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V, S> IntoIterator for IndexMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = inner::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;

    #[test]
    fn smoke() -> Result<()> {
        let mut map = IndexMap::new();

        map.insert("a", 10)?;
        map.insert("b", 20)?;
        map.insert("c", 30)?;

        assert_eq!(map[&"a"], 10);
        assert_eq!(map[&"b"], 20);
        assert_eq!(map[&"c"], 30);

        assert_eq!(map[0], 10);
        assert_eq!(map[1], 20);
        assert_eq!(map[2], 30);

        Ok(())
    }
}
