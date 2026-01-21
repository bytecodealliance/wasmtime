use cranelift_entity::{EntityRef, Keys, SetIter};
use wasmtime_error::OutOfMemory;

/// Like `cranelift_entity::EntitySet` but enforces fallible allocation for all
/// methods that allocate.
#[derive(Debug, Default)]
pub struct EntitySet<K>
where
    K: EntityRef,
{
    inner: cranelift_entity::EntitySet<K>,
}

impl<K> EntitySet<K>
where
    K: EntityRef,
{
    /// Create a new empty set.
    pub fn new() -> Self {
        EntitySet {
            inner: Default::default(),
        }
    }

    /// Creates a new empty set with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Result<Self, OutOfMemory> {
        let mut set = Self::new();
        set.inner.try_ensure_capacity(capacity)?;
        Ok(set)
    }

    /// Ensure that there is enough capacity to hold `capacity` total elements.
    pub fn ensure_capacity(&mut self, capacity: usize) -> Result<(), OutOfMemory> {
        self.inner.try_ensure_capacity(capacity)
    }

    /// Is this set completely empty?
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the element at `k` if it exists.
    pub fn contains(&self, k: K) -> bool {
        self.inner.contains(k)
    }

    /// Remove all entries from this set.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Iterate over all the keys up to the maximum in this set.
    ///
    /// This will yield intermediate keys on the way up to the max key, even if
    /// they are not contained within the set.
    pub fn keys(&self) -> Keys<K> {
        self.inner.keys()
    }

    /// Iterate over the elements of this set.
    pub fn iter(&self) -> SetIter<'_, K> {
        self.inner.iter()
    }

    /// Insert the element at `k`.
    ///
    /// Returns `true` if `k` was not present in the set, i.e. this is a
    /// newly-added element. Returns `false` otherwise.
    pub fn insert(&mut self, k: K) -> Result<bool, OutOfMemory> {
        self.inner.try_ensure_capacity(k.index())?;
        Ok(self.inner.insert(k))
    }

    /// Remove `k` from this bitset.
    ///
    /// Returns whether `k` was previously in this set or not.
    pub fn remove(&mut self, k: K) -> bool {
        self.inner.remove(k)
    }

    /// Removes and returns the highest-index entity from the set if it exists.
    pub fn pop(&mut self) -> Option<K> {
        self.inner.pop()
    }
}
