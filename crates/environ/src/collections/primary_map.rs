use core::ops::{Index, IndexMut};
use cranelift_entity::EntityRef;
use wasmtime_error::OutOfMemory;

/// Like `cranelift_entity::PrimaryMap` but enforces fallible allocation for all
/// methods that allocate.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PrimaryMap<K, V>
where
    K: EntityRef,
{
    inner: cranelift_entity::PrimaryMap<K, V>,
}

impl<K, V> PrimaryMap<K, V>
where
    K: EntityRef,
{
    /// Create a new empty map.
    pub fn new() -> Self {
        Self {
            inner: cranelift_entity::PrimaryMap::new(),
        }
    }

    /// Create a new empty map with the given capacity.
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut map = Self::new();
        map.reserve(capacity)?;
        Ok(map)
    }

    /// Check if `k` is a valid key in the map.
    pub fn is_valid(&self, k: K) -> bool {
        self.inner.is_valid(k)
    }

    /// Get the element at `k` if it exists.
    pub fn get(&self, k: K) -> Option<&V> {
        self.inner.get(k)
    }

    /// Get the slice of values associated with the given range of keys, if any.
    pub fn get_range(&self, range: core::ops::Range<K>) -> Option<&[V]> {
        self.inner.get_range(range)
    }

    /// Get the element at `k` if it exists, mutable version.
    pub fn get_mut(&mut self, k: K) -> Option<&mut V> {
        self.inner.get_mut(k)
    }

    /// Is this map completely empty?
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the total number of entity references created.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Iterate over all the keys in this map.
    pub fn keys(&self) -> cranelift_entity::Keys<K> {
        self.inner.keys()
    }

    /// Iterate over all the values in this map.
    pub fn values(&self) -> core::slice::Iter<'_, V> {
        self.inner.values()
    }

    /// Iterate over all the values in this map, mutable edition.
    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, V> {
        self.inner.values_mut()
    }

    /// Get this map's underlying values as a slice.
    pub fn as_values_slice(&self) -> &[V] {
        self.inner.as_values_slice()
    }

    /// Iterate over all the keys and values in this map.
    pub fn iter(&self) -> cranelift_entity::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Iterate over all the keys and values in this map, mutable edition.
    pub fn iter_mut(&mut self) -> cranelift_entity::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }

    /// Remove all entries from this map.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Get the key that will be assigned to the next pushed value.
    pub fn next_key(&self) -> K {
        self.inner.next_key()
    }

    /// Append `v` to the mapping, assigning a new key which is returned.
    pub fn push(&mut self, v: V) -> Result<K, OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.push(v))
    }

    /// Returns the last element that was inserted in the map.
    pub fn last(&self) -> Option<(K, &V)> {
        self.inner.last()
    }

    /// Returns the last element that was inserted in the map.
    pub fn last_mut(&mut self) -> Option<(K, &mut V)> {
        self.inner.last_mut()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    pub fn reserve(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve(additional)
    }

    /// Reserves the minimum capacity for exactly `additional` more elements to be inserted.
    pub fn reserve_exact(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve_exact(additional)
    }

    /// Returns mutable references to many elements at once.
    ///
    /// Returns an error if an element does not exist, or if the same key was passed more than
    /// once.
    pub fn get_disjoint_mut<const N: usize>(
        &mut self,
        indices: [K; N],
    ) -> Result<[&mut V; N], core::slice::GetDisjointMutError> {
        self.inner.get_disjoint_mut(indices)
    }

    /// Performs a binary search on the values with a key extraction function.
    ///
    /// Assumes that the values are sorted by the key extracted by the function.
    ///
    /// If the value is found then `Ok(K)` is returned, containing the entity key
    /// of the matching value.
    ///
    /// If there are multiple matches, then any one of the matches could be returned.
    ///
    /// If the value is not found then Err(K) is returned, containing the entity key
    /// where a matching element could be inserted while maintaining sorted order.
    pub fn binary_search_values_by_key<'a, B, F>(&'a self, b: &B, f: F) -> Result<K, K>
    where
        F: FnMut(&'a V) -> B,
        B: Ord,
    {
        self.inner.binary_search_values_by_key(b, f)
    }

    /// Analog of `get_raw` except that a raw pointer is returned rather than a
    /// mutable reference.
    ///
    /// The default accessors of items in [`PrimaryMap`] will invalidate all
    /// previous borrows obtained from the map according to miri. This function
    /// can be used to acquire a pointer and then subsequently acquire a second
    /// pointer later on without invalidating the first one. In other words
    /// this is only here to help borrow two elements simultaneously with miri.
    pub fn get_raw_mut(&mut self, k: K) -> Option<*mut V> {
        self.inner.get_raw_mut(k)
    }
}

impl<K, V> Index<K> for PrimaryMap<K, V>
where
    K: EntityRef,
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        &self.inner[k]
    }
}

impl<K, V> IndexMut<K> for PrimaryMap<K, V>
where
    K: EntityRef,
{
    fn index_mut(&mut self, k: K) -> &mut V {
        &mut self.inner[k]
    }
}
