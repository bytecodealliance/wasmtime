use crate::error::OutOfMemory;
use core::{fmt, ops::Index};
use cranelift_entity::{EntityRef, SecondaryMap as Inner};

/// Like [`cranelift_entity::SecondaryMap`] but all allocation is fallible.
pub struct SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    inner: Inner<K, V>,
}

impl<K, V> fmt::Debug for SecondaryMap<K, V>
where
    K: EntityRef + fmt::Debug,
    V: fmt::Debug + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<K, V> SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    /// Same as [`cranelift_entity::SecondaryMap::new`].
    pub fn new() -> Self
    where
        V: Default,
    {
        Self {
            inner: Inner::new(),
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::try_with_capacity`].
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory>
    where
        V: Default,
    {
        Ok(Self {
            inner: Inner::try_with_capacity(capacity)?,
        })
    }

    /// Same as [`cranelift_entity::SecondaryMap::with_default`].
    pub fn with_default(default: V) -> Self {
        Self {
            inner: Inner::with_default(default),
        }
    }

    /// Same as [`cranelift_entity::SecondaryMap::capacity`].
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Same as [`cranelift_entity::SecondaryMap::get`].
    pub fn get(&self, k: K) -> Option<&V> {
        self.inner.get(k)
    }

    /// Same as [`cranelift_entity::SecondaryMap::get_mut`].
    pub fn get_mut(&mut self, k: K) -> Option<&mut V> {
        self.inner.get_mut(k)
    }

    /// Same as [`cranelift_entity::SecondaryMap::try_insert`].
    pub fn insert(&mut self, k: K, v: V) -> Result<Option<V>, OutOfMemory> {
        self.inner.try_insert(k, v)
    }

    /// Same as [`cranelift_entity::SecondaryMap::remove`].
    pub fn remove(&mut self, k: K) -> Option<V> {
        self.inner.remove(k)
    }

    /// Same as [`cranelift_entity::SecondaryMap::is_empty`].
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Same as [`cranelift_entity::SecondaryMap::clear`].
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Same as [`cranelift_entity::SecondaryMap::iter`].
    pub fn iter(&self) -> cranelift_entity::Iter<'_, K, V> {
        self.inner.iter()
    }

    /// Same as [`cranelift_entity::SecondaryMap::iter_mut`].
    pub fn iter_mut(&mut self) -> cranelift_entity::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }

    /// Same as [`cranelift_entity::SecondaryMap::keys`].
    pub fn keys(&self) -> cranelift_entity::Keys<K> {
        self.inner.keys()
    }

    /// Same as [`cranelift_entity::SecondaryMap::values`].
    pub fn values(&self) -> core::slice::Iter<'_, V> {
        self.inner.values()
    }

    /// Same as [`cranelift_entity::SecondaryMap::values_mut`].
    pub fn values_mut(&mut self) -> core::slice::IterMut<'_, V> {
        self.inner.values_mut()
    }

    /// Resize the map to have `n` entries by adding default entries as needed.
    pub fn resize(&mut self, n: usize) -> Result<(), OutOfMemory> {
        self.inner.try_resize(n)
    }
}

impl<K, V> Default for SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone + Default,
{
    fn default() -> SecondaryMap<K, V> {
        SecondaryMap::new()
    }
}

// NB: no `IndexMut` implementation because it requires allocation but the trait
// doesn't allow for fallibility.
impl<K, V> Index<K> for SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        &self.inner[k]
    }
}
