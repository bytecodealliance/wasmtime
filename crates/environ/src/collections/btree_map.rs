//! OOM-handling `TryBTreeMap` implementation.

use crate::error::OutOfMemory;
use core::{mem, ops::RangeBounds, ptr::NonNull};
use wasmtime_core::slab::{Id, Slab};

/// Like `std::collections::BTreeMap` but its methods return errors on
/// allocation failure.
pub struct TryBTreeMap<K, V>
where
    K: Copy,
{
    values: Slab<V>,
    forest: cranelift_bforest::MapForest<K, Id>,
    map: cranelift_bforest::Map<K, Id>,
}

impl<K, V> Default for TryBTreeMap<K, V>
where
    K: Copy,
{
    fn default() -> Self {
        Self {
            values: Default::default(),
            forest: cranelift_bforest::MapForest::new(),
            map: Default::default(),
        }
    }
}

impl<K, V> TryBTreeMap<K, V>
where
    K: Copy,
{
    /// Same as [`std::collections::BTreeMap::new`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Same as [`std::collections::BTreeMap::len`].
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Same as [`std::collections::BTreeMap::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn get_id(&self, key: K) -> Option<Id>
    where
        K: Ord,
    {
        self.map.get(key, &self.forest, &())
    }

    /// Same as [`std::collections::BTreeMap::contains_key`].
    pub fn contains_key(&self, key: K) -> bool
    where
        K: Ord,
    {
        self.get_id(key).is_some()
    }

    /// Same as [`std::collections::BTreeMap::get`].
    pub fn get(&self, key: K) -> Option<&V>
    where
        K: Ord,
    {
        let id = self.get_id(key)?;
        Some(&self.values[id])
    }

    /// Same as [`std::collections::BTreeMap::get_mut`].
    pub fn get_mut(&mut self, key: K) -> Option<&mut V>
    where
        K: Ord,
    {
        let id = self.get_id(key)?;
        Some(&mut self.values[id])
    }

    /// Same as [`std::collections::BTreeMap::insert`] but returns an error on
    /// allocation failure.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, OutOfMemory>
    where
        K: Ord,
    {
        if let Some(id) = self.get_id(key) {
            return Ok(Some(mem::replace(&mut self.values[id], value)));
        }

        let id = self.values.alloc(value)?;
        match self.map.try_insert(key, id, &mut self.forest, &()) {
            Ok(old) => {
                debug_assert!(old.is_none());
                Ok(None)
            }
            Err(oom) => {
                self.values.dealloc(id);
                Err(oom)
            }
        }
    }

    /// Same as [`std::collections::BTreeMap::remove`].
    pub fn remove(&mut self, key: K) -> Option<V>
    where
        K: Ord,
    {
        let id = self.map.remove(key, &mut self.forest, &())?;
        Some(self.values.dealloc(id))
    }

    /// Same as [`std::collections::BTreeMap::clear`].
    ///
    /// Does not deallocate the underlying storage.
    pub fn clear(&mut self) {
        self.values.clear();
        // NB: Do not do `self.map.clear(&mut self.forest)` because that is
        // designed to work in scenarios where multiple maps are sharing the
        // same forest (which we are not doing here) and will actually traverse
        // the b-tree's nodes and deallocate each of them one at a time. For our
        // single-map-in-the-forest case, it is equivalent, but much faster, to
        // simply clear the forest itself and reset the map to its default,
        // empty state.
        self.forest.clear();
        self.map = Default::default();
    }

    /// Same as [`std::collections::BTreeMap::iter`].
    pub fn iter(&self) -> BTreeMapIter<'_, K, V> {
        BTreeMapIter {
            inner: self.map.iter(&self.forest),
            values: &self.values,
        }
    }

    /// Same as [`std::collections::BTreeMap::iter`].
    pub fn iter_mut(&mut self) -> BTreeMapIterMut<'_, K, V> {
        BTreeMapIterMut {
            inner: self.map.iter(&self.forest),
            values: &mut self.values,
        }
    }

    /// Same as [`std::collections::BTreeMap::keys`].
    pub fn keys(&self) -> BTreeMapKeys<'_, K, V> {
        BTreeMapKeys { inner: self.iter() }
    }

    /// Same as [`std::collections::BTreeMap::values`].
    pub fn values(&self) -> BTreeMapValues<'_, K, V> {
        BTreeMapValues { inner: self.iter() }
    }

    /// Same as [`std::collections::BTreeMap::values_mut`].
    pub fn values_mut(&mut self) -> BTreeMapValuesMut<'_, K, V> {
        BTreeMapValuesMut {
            inner: self.iter_mut(),
        }
    }

    /// Same as [`std::collections::BTreeMap::range`].
    pub fn range<R>(&self, range: R) -> BTreeMapRange<'_, K, V>
    where
        K: Ord,
        R: RangeBounds<K>,
    {
        BTreeMapRange {
            inner: self.map.range(range, &self.forest, &()),
            values: &self.values,
        }
    }

    /// Same as [`std::collections::BTreeMap::range_mut`].
    pub fn range_mut<R>(&mut self, range: R) -> BTreeMapRangeMut<'_, K, V>
    where
        K: Ord,
        R: RangeBounds<K>,
    {
        BTreeMapRangeMut {
            inner: self.map.range(range, &self.forest, &()),
            values: &mut self.values,
        }
    }

    /// Same as [`std::collections::BTreeMap::entry`].
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V>
    where
        K: Ord,
    {
        let mut cursor = self.map.cursor_mut(&mut self.forest, &());
        match cursor.goto(key) {
            Some(_) => Entry::Occupied(OccupiedEntry {
                cursor,
                values: &mut self.values,
            }),
            None => Entry::Vacant(VacantEntry {
                key,
                cursor,
                values: &mut self.values,
            }),
        }
    }
}

impl<'a, K, V> IntoIterator for &'a TryBTreeMap<K, V>
where
    K: Copy,
{
    type Item = (K, &'a V);
    type IntoIter = BTreeMapIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over `(K, &V)` pairs returned by [`TryBTreeMap::iter`].
pub struct BTreeMapIter<'a, K, V>
where
    K: Copy,
{
    inner: cranelift_bforest::MapIter<'a, K, Id>,
    values: &'a Slab<V>,
}

impl<'a, K, V> Iterator for BTreeMapIter<'a, K, V>
where
    K: Copy,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, id) = self.inner.next()?;
        Some((key, &self.values[id]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> IntoIterator for &'a mut TryBTreeMap<K, V>
where
    K: Copy,
{
    type Item = (K, &'a mut V);
    type IntoIter = BTreeMapIterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over `(K, &mut V)` pairs returned by [`TryBTreeMap::iter_mut`].
pub struct BTreeMapIterMut<'a, K, V>
where
    K: Copy,
{
    inner: cranelift_bforest::MapIter<'a, K, Id>,
    values: &'a mut Slab<V>,
}

impl<'a, K, V> Iterator for BTreeMapIterMut<'a, K, V>
where
    K: Copy,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, id) = self.inner.next()?;
        let val = &mut self.values[id];

        // Safety: It is okay to extend the borrow from `&'1 mut self` to `&'a`
        // because each entry is associated with a unique `id` and we will never
        // return multiple mutable borrows of the same value.
        let val = unsafe { NonNull::from(val).as_mut() };

        Some((key, val))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> IntoIterator for TryBTreeMap<K, V>
where
    K: Copy + Ord,
{
    type Item = (K, V);
    type IntoIter = BTreeMapIntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        BTreeMapIntoIter {
            inner: self.map.into_iter(self.forest),
            values: self.values,
        }
    }
}

/// An iterator over `(K, V)` pairs returned by [`TryBTreeMap::into_iter`].
pub struct BTreeMapIntoIter<K, V>
where
    K: Copy,
{
    inner: cranelift_bforest::MapIntoIter<K, Id>,
    values: Slab<V>,
}

impl<K, V> Iterator for BTreeMapIntoIter<K, V>
where
    K: Copy + Ord,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, id) = self.inner.next()?;
        let value = self.values.dealloc(id);
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.values.len();
        (len, Some(len))
    }
}

/// An iterator over keys returned by [`TryBTreeMap::keys`].
pub struct BTreeMapKeys<'a, K, V>
where
    K: Copy,
{
    inner: BTreeMapIter<'a, K, V>,
}

impl<'a, K, V> Iterator for BTreeMapKeys<'a, K, V>
where
    K: Copy,
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        let (k, _v) = self.inner.next()?;
        Some(k)
    }
}

/// An iterator over shared values returned by [`TryBTreeMap::values`].
pub struct BTreeMapValues<'a, K, V>
where
    K: Copy,
{
    inner: BTreeMapIter<'a, K, V>,
}

impl<'a, K, V> Iterator for BTreeMapValues<'a, K, V>
where
    K: Copy,
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        let (_k, v) = self.inner.next()?;
        Some(v)
    }
}

/// An iterator over mutable values returned by [`TryBTreeMap::values_mut`].
pub struct BTreeMapValuesMut<'a, K, V>
where
    K: Copy,
{
    inner: BTreeMapIterMut<'a, K, V>,
}

impl<'a, K, V> Iterator for BTreeMapValuesMut<'a, K, V>
where
    K: Copy,
{
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        let (_k, v) = self.inner.next()?;
        Some(v)
    }
}

/// A range iterator of `(K, &'a V)` items returned by [`TryBTreeMap::range`].
pub struct BTreeMapRange<'a, K, V>
where
    K: Copy + Ord,
{
    inner: cranelift_bforest::MapRange<'a, K, Id, ()>,
    values: &'a Slab<V>,
}

impl<'a, K, V> Iterator for BTreeMapRange<'a, K, V>
where
    K: Copy + Ord,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, id) = self.inner.next()?;
        Some((key, &self.values[id]))
    }
}

impl<'a, K, V> DoubleEndedIterator for BTreeMapRange<'a, K, V>
where
    K: Copy + Ord,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let (key, id) = self.inner.next_back()?;
        Some((key, &self.values[id]))
    }
}

/// A range iterator of `(K, &'a V)` items returned by [`TryBTreeMap::range`].
pub struct BTreeMapRangeMut<'a, K, V>
where
    K: Copy + Ord,
{
    inner: cranelift_bforest::MapRange<'a, K, Id, ()>,
    values: &'a mut Slab<V>,
}

impl<'a, K, V> Iterator for BTreeMapRangeMut<'a, K, V>
where
    K: Copy + Ord,
{
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, id) = self.inner.next()?;
        let val = &mut self.values[id];

        // Safety: It is okay to extend the borrow from `&'1 mut self` to `&'a`
        // because each entry is associated with a unique `id` and we will never
        // return multiple mutable borrows of the same value.
        let val = unsafe { NonNull::from(val).as_mut() };

        Some((key, val))
    }
}

/// Same as [`std::collections::btree_map::Entry`].
#[allow(missing_docs, reason = "self explanatory")]
pub enum Entry<'a, K, V>
where
    K: Copy + Ord,
{
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: Copy + Ord,
{
    /// Same as [`std::collections::btree_map::Entry::or_insert`] but returns an
    /// error on allocation failure.
    pub fn or_insert(self, default: V) -> Result<&'a mut V, OutOfMemory> {
        self.or_insert_with(|| default)
    }

    /// Same as [`std::collections::btree_map::Entry::or_insert_with`] but
    /// returns an error on allocation failure.
    pub fn or_insert_with<F>(self, default: F) -> Result<&'a mut V, OutOfMemory>
    where
        F: FnOnce() -> V,
    {
        self.or_insert_with_key(|_| default())
    }

    /// Same as [`std::collections::btree_map::Entry::or_insert_with_key`] but
    /// returns an error on allocation failure.
    pub fn or_insert_with_key<F>(self, default: F) -> Result<&'a mut V, OutOfMemory>
    where
        F: FnOnce(K) -> V,
    {
        match self {
            Entry::Occupied(e) => {
                let id = e.cursor.value().unwrap();
                Ok(&mut e.values[id])
            }
            Entry::Vacant(mut e) => {
                let id = e.values.alloc(default(e.key))?;
                match e.cursor.try_insert(e.key, id) {
                    Ok(old) => {
                        debug_assert!(old.is_none());
                        Ok(&mut e.values[id])
                    }
                    Err(oom) => {
                        e.values.dealloc(id);
                        Err(oom)
                    }
                }
            }
        }
    }

    /// Same as [`std::collections::btree_map::Entry::key`].
    pub fn key(&self) -> K {
        match self {
            Entry::Occupied(e) => e.cursor.key().unwrap(),
            Entry::Vacant(e) => e.key,
        }
    }

    /// Same as [`std::collections::btree_map::Entry::and_modify`].
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Entry::Occupied(mut e) => {
                f(e.get_mut());
                Entry::Occupied(e)
            }
            e @ Entry::Vacant(_) => e,
        }
    }

    /// Same as [`std::collections::btree_map::Entry::and_modify`] but returns
    /// an error on allocation failure.
    pub fn insert_entry(self, value: V) -> Result<OccupiedEntry<'a, K, V>, OutOfMemory> {
        match self {
            Entry::Occupied(e) => Ok(e),
            Entry::Vacant(e) => e.insert_entry(value),
        }
    }

    /// Same as [`std::collections::btree_map::Entry::or_default`] but returns
    /// an error on allocation failure.
    pub fn or_default(self) -> Result<&'a mut V, OutOfMemory>
    where
        V: Default,
    {
        self.or_insert_with(Default::default)
    }
}

/// Same as [`std::collections::btree_map::OccupiedEntry`].
pub struct OccupiedEntry<'a, K, V>
where
    K: Copy + Ord,
{
    cursor: cranelift_bforest::MapCursorMut<'a, K, Id, ()>,
    values: &'a mut Slab<V>,
}

impl<'a, K, V> OccupiedEntry<'a, K, V>
where
    K: Copy + Ord,
{
    /// Same as [`std::collections::btree_map::OccupiedEntry::key`].
    pub fn key(&self) -> K {
        self.cursor.key().unwrap()
    }

    /// Same as [`std::collections::btree_map::OccupiedEntry::remove_entry`].
    pub fn remove_entry(mut self) -> (K, V) {
        let key = self.key();
        let id = self.cursor.remove().unwrap();
        let value = self.values.dealloc(id);
        (key, value)
    }

    /// Same as [`std::collections::btree_map::OccupiedEntry::get`].
    pub fn get(&self) -> &V {
        let id = self.cursor.value().unwrap();
        &self.values[id]
    }

    /// Same as [`std::collections::btree_map::OccupiedEntry::get_mut`].
    pub fn get_mut(&mut self) -> &mut V {
        let id = self.cursor.value().unwrap();
        &mut self.values[id]
    }

    /// Same as [`std::collections::btree_map::OccupiedEntry::into_mut`].
    pub fn into_mut(self) -> &'a mut V {
        let id = self.cursor.value().unwrap();
        &mut self.values[id]
    }

    /// Same as [`std::collections::btree_map::OccupiedEntry::insert`].
    pub fn insert(self, value: V) -> V {
        let id = self.cursor.value().unwrap();
        mem::replace(&mut self.values[id], value)
    }

    /// Same as [`std::collections::btree_map::OccupiedEntry::remove`].
    pub fn remove(mut self) -> V {
        let id = self.cursor.remove().unwrap();
        self.values.dealloc(id)
    }
}

/// Same as [`std::collections::btree_map::VacantEntry`].
pub struct VacantEntry<'a, K, V>
where
    K: Copy + Ord,
{
    key: K,
    cursor: cranelift_bforest::MapCursorMut<'a, K, Id, ()>,
    values: &'a mut Slab<V>,
}

impl<'a, K, V> VacantEntry<'a, K, V>
where
    K: Copy + Ord,
{
    /// Same as [`std::collections::btree_map::VacantEntry::key`].
    pub fn key(&self) -> K {
        self.key
    }

    /// Same as [`std::collections::btree_map::VacantEntry::into_key`].
    pub fn into_key(self) -> K {
        self.key
    }

    /// Same as [`std::collections::btree_map::VacantEntry::insert`] but returns
    /// an error on allocation failure.
    pub fn insert(self, value: V) -> Result<&'a mut V, OutOfMemory> {
        Ok(self.insert_entry(value)?.into_mut())
    }

    /// Same as [`std::collections::btree_map::VacantEntry::insert_entry`] but
    /// returns an error on allocation failure.
    pub fn insert_entry(mut self, value: V) -> Result<OccupiedEntry<'a, K, V>, OutOfMemory> {
        let id = self.values.alloc(value)?;
        match self.cursor.try_insert(self.key, id) {
            Ok(old) => {
                debug_assert!(old.is_none());
                Ok(OccupiedEntry {
                    cursor: self.cursor,
                    values: self.values,
                })
            }
            Err(oom) => {
                self.values.dealloc(id);
                Err(oom)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use alloc::{vec, vec::Vec};

    type M = TryBTreeMap<usize, f32>;

    #[test]
    fn new() -> Result<()> {
        let m = M::new();
        assert!(m.is_empty());
        Ok(())
    }

    #[test]
    fn len() -> Result<()> {
        let mut m = M::new();
        for i in 0..10 {
            assert_eq!(m.len(), i);
            m.insert(i, i as f32)?;
        }
        assert_eq!(m.len(), 10);
        for i in 0..10 {
            assert_eq!(m.len(), 10 - i);
            m.remove(i);
        }
        assert_eq!(m.len(), 0);
        Ok(())
    }

    #[test]
    fn is_empty() -> Result<()> {
        let mut m = M::new();
        assert!(m.is_empty());
        m.insert(42, 42.0)?;
        assert!(!m.is_empty());
        m.remove(42);
        assert!(m.is_empty());
        Ok(())
    }

    #[test]
    fn contains_key() -> Result<()> {
        let mut m = M::new();
        assert!(!m.contains_key(36));
        assert!(!m.contains_key(42));
        m.insert(42, 42.0)?;
        assert!(!m.contains_key(36));
        assert!(m.contains_key(42));
        m.remove(42);
        assert!(!m.contains_key(36));
        assert!(!m.contains_key(42));
        Ok(())
    }

    #[test]
    fn get() -> Result<()> {
        let mut m = M::new();
        assert!(m.get(36).is_none());
        assert!(m.get(42).is_none());
        m.insert(42, 42.0)?;
        assert!(m.get(36).is_none());
        assert_eq!(m.get(42), Some(&42.0));
        m.remove(42);
        assert!(m.get(36).is_none());
        assert!(m.get(42).is_none());
        Ok(())
    }

    #[test]
    fn get_mut() -> Result<()> {
        let mut m = M::new();
        assert!(m.get_mut(36).is_none());
        assert!(m.get_mut(42).is_none());
        m.insert(42, 42.0)?;
        assert!(m.get_mut(36).is_none());
        assert_eq!(m.get_mut(42).copied(), Some(42.0));
        *m.get_mut(42).unwrap() = 99.0;
        assert_eq!(m.get_mut(42).copied(), Some(99.0));
        m.remove(42);
        assert!(m.get_mut(36).is_none());
        assert!(m.get_mut(42).is_none());
        Ok(())
    }

    #[test]
    fn insert() -> Result<()> {
        let mut m = M::new();
        let old = m.insert(11, 0.0)?;
        assert!(old.is_none());
        let old = m.insert(11, 1.0)?;
        assert_eq!(old, Some(0.0));
        Ok(())
    }

    #[test]
    fn remove() -> Result<()> {
        let mut m = M::new();
        let old = m.remove(10);
        assert!(old.is_none());
        m.insert(10, 123.0)?;
        let old = m.remove(10);
        assert_eq!(old, Some(123.0));
        Ok(())
    }

    #[test]
    fn clear() -> Result<()> {
        let mut m = M::new();
        for i in 0..10 {
            m.insert(i, i as f32)?;
        }
        m.clear();
        assert!(m.is_empty());
        for i in 0..10 {
            assert!(m.get(i).is_none());
        }
        Ok(())
    }

    #[test]
    fn iter() -> Result<()> {
        let mut m = M::new();
        for i in 0..5 {
            m.insert(i, i as f32)?;
        }
        assert_eq!(
            m.iter().collect::<Vec<_>>(),
            vec![(0, &0.0), (1, &1.0), (2, &2.0), (3, &3.0), (4, &4.0)],
        );
        Ok(())
    }

    #[test]
    fn iter_mut() -> Result<()> {
        let mut m = M::new();
        for i in 0..5 {
            m.insert(i, i as f32)?;
        }
        assert_eq!(
            m.iter_mut()
                .map(|(k, v)| (k, mem::replace(v, 0.0)))
                .collect::<Vec<_>>(),
            vec![(0, 0.0), (1, 1.0), (2, 2.0), (3, 3.0), (4, 4.0)],
        );
        for i in 0..5 {
            assert_eq!(m.get(i), Some(&0.0));
        }
        Ok(())
    }

    #[test]
    fn keys() -> Result<()> {
        let mut m = M::new();
        for i in 0..5 {
            m.insert(i, i as f32)?;
        }
        assert_eq!(m.keys().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
        Ok(())
    }

    #[test]
    fn values() -> Result<()> {
        let mut m = M::new();
        for i in 0..5 {
            m.insert(i, i as f32)?;
        }
        assert_eq!(
            m.values().collect::<Vec<_>>(),
            vec![&0.0, &1.0, &2.0, &3.0, &4.0],
        );
        Ok(())
    }

    #[test]
    fn values_mut() -> Result<()> {
        let mut m = M::new();
        for i in 0..5 {
            m.insert(i, i as f32)?;
        }
        assert_eq!(
            m.values_mut()
                .map(|v| mem::replace(v, 0.0))
                .collect::<Vec<_>>(),
            vec![0.0, 1.0, 2.0, 3.0, 4.0],
        );
        assert_eq!(
            m.values().collect::<Vec<_>>(),
            vec![&0.0, &0.0, &0.0, &0.0, &0.0],
        );
        Ok(())
    }

    #[test]
    fn range() -> Result<()> {
        let mut m = M::new();
        for i in 0..5 {
            m.insert(i, i as f32)?;
        }
        assert_eq!(m.range(..2).collect::<Vec<_>>(), vec![(0, &0.0), (1, &1.0)]);
        assert_eq!(m.range(3..).collect::<Vec<_>>(), vec![(3, &3.0), (4, &4.0)]);
        assert_eq!(
            m.range(1..3).collect::<Vec<_>>(),
            vec![(1, &1.0), (2, &2.0)],
        );
        assert_eq!(
            m.range(2..=3).collect::<Vec<_>>(),
            vec![(2, &2.0), (3, &3.0)],
        );
        assert_eq!(m.range(5..).collect::<Vec<_>>(), vec![]);
        assert_eq!(m.range(..0).collect::<Vec<_>>(), vec![]);
        Ok(())
    }

    #[test]
    fn range_mut() -> Result<()> {
        let mut m = M::new();
        for i in 0..5 {
            m.insert(i, i as f32)?;
        }
        assert_eq!(
            m.range_mut(1..3)
                .map(|(k, v)| (k, mem::replace(v, 99.0)))
                .collect::<Vec<_>>(),
            vec![(1, 1.0), (2, 2.0)],
        );
        assert_eq!(
            m.values().copied().collect::<Vec<_>>(),
            vec![0.0, 99.0, 99.0, 3.0, 4.0]
        );
        Ok(())
    }

    #[test]
    fn entry() -> Result<()> {
        let mut m = M::new();

        let v = m.entry(0).or_insert(99.0)?;
        assert_eq!(*v, 99.0);

        let v = m.entry(0).or_insert(0.0)?;
        assert_eq!(*v, 99.0);

        let e = match m.entry(1) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(e) => e.insert_entry(42.0)?,
        };
        assert_eq!(e.key(), 1);
        assert_eq!(e.get(), &42.0);
        let old = e.insert(43.0);
        assert_eq!(old, 42.0);
        assert_eq!(m.get(1), Some(&43.0));

        let e = match m.entry(1) {
            Entry::Occupied(e) => e,
            Entry::Vacant(_) => unreachable!(),
        };
        assert_eq!(e.key(), 1);
        assert_eq!(e.get(), &43.0);
        let (k, v) = e.remove_entry();
        assert_eq!(k, 1);
        assert_eq!(v, 43.0);
        assert!(m.get(1).is_none());

        let e = match m.entry(2) {
            Entry::Occupied(_) => unreachable!(),
            Entry::Vacant(e) => e,
        };
        assert_eq!(e.key(), 2);
        let v = e.insert(99.0)?;
        assert_eq!(*v, 99.0);

        Ok(())
    }
}
