use core::ops::{Index, IndexMut};
use cranelift_entity::EntityRef;
use wasmtime_core::error::OutOfMemory;

/// Like [`cranelift_entity::PrimaryMap`] but enforces fallible allocation for
/// all methods that allocate.
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
    /// Same as [`cranelift_entity::PrimaryMap::new`].
    pub fn new() -> Self {
        Self {
            inner: cranelift_entity::PrimaryMap::new(),
        }
    }

    /// Same as [`cranelift_entity::PrimaryMap::try_with_capacity`].
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut map = Self::new();
        map.reserve(capacity)?;
        Ok(map)
    }

    /// Same as [`cranelift_entity::PrimaryMap::is_valid`].
    pub fn is_valid(&self, k: K) -> bool {
        self.inner.is_valid(k)
    }

    /// Same as [`cranelift_entity::PrimaryMap::get`].
    pub fn get(&self, k: K) -> Option<&V> {
        self.inner.get(k)
    }

    /// Same as [`cranelift_entity::PrimaryMap::get_range`].
    pub fn get_range(&self, range: core::ops::Range<K>) -> Option<&[V]> {
        self.inner.get_range(range)
    }

    /// Same as [`cranelift_entity::PrimaryMap::get_mut`].
    pub fn get_mut(&mut self, k: K) -> Option<&mut V> {
        self.inner.get_mut(k)
    }

    /// Same as [`cranelift_entity::PrimaryMap::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`cranelift_entity::PrimaryMap::len`].
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Same as [`cranelift_entity::PrimaryMap::keys`].
    pub fn keys(&self) -> cranelift_entity::Keys<K> {
        self.inner.keys()
    }

    /// Same as [`cranelift_entity::PrimaryMap::values`].
    pub fn values(&self) -> core::slice::Iter<'_, V> {
        self.inner.values()
    }

    /// Same as [`cranelift_entity::PrimaryMap::values_mut`].
    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, V> {
        self.inner.values_mut()
    }

    /// Same as [`cranelift_entity::PrimaryMap::as_values_slice`].
    pub fn as_values_slice(&self) -> &[V] {
        self.inner.as_values_slice()
    }

    /// Same as [`cranelift_entity::PrimaryMap::iter`].
    pub fn iter(&self) -> cranelift_entity::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Same as [`cranelift_entity::PrimaryMap::iter_mut`].
    pub fn iter_mut(&mut self) -> cranelift_entity::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }

    /// Same as [`cranelift_entity::PrimaryMap::clear`].
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Same as [`cranelift_entity::PrimaryMap::next_key`].
    pub fn next_key(&self) -> K {
        self.inner.next_key()
    }

    /// Same as [`cranelift_entity::PrimaryMap::push`] but returns an error on
    /// allocation failure.
    pub fn push(&mut self, v: V) -> Result<K, OutOfMemory> {
        self.reserve(1)?;
        Ok(self.inner.push(v))
    }

    /// Same as [`cranelift_entity::PrimaryMap::last`].
    pub fn last(&self) -> Option<(K, &V)> {
        self.inner.last()
    }

    /// Same as [`cranelift_entity::PrimaryMap::last_mut`].
    pub fn last_mut(&mut self) -> Option<(K, &mut V)> {
        self.inner.last_mut()
    }

    /// Same as [`cranelift_entity::PrimaryMap::try_reserve`].
    pub fn reserve(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve(additional)
    }

    /// Same as [`cranelift_entity::PrimaryMap::try_reserve_exact`].
    pub fn reserve_exact(&mut self, additional: usize) -> Result<(), OutOfMemory> {
        self.inner.try_reserve_exact(additional)
    }

    /// Same as [`cranelift_entity::PrimaryMap::get_disjoint_mut`].
    pub fn get_disjoint_mut<const N: usize>(
        &mut self,
        indices: [K; N],
    ) -> Result<[&mut V; N], core::slice::GetDisjointMutError> {
        self.inner.get_disjoint_mut(indices)
    }

    /// Same as [`cranelift_entity::PrimaryMap::binary_search_values_by_key`].
    pub fn binary_search_values_by_key<'a, B, F>(&'a self, b: &B, f: F) -> Result<K, K>
    where
        F: FnMut(&'a V) -> B,
        B: Ord,
    {
        self.inner.binary_search_values_by_key(b, f)
    }

    /// Same as [`cranelift_entity::PrimaryMap::get_raw_mut`].
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
