//! Densely numbered entity references as mapping keys.
use entity::{EntityRef, Keys};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// A primary mapping `K -> V` allocating dense entity references.
///
/// The `PrimaryMap` data structure uses the dense index space to implement a map with a vector.
///
/// A primary map contains the main definition of an entity, and it can be used to allocate new
/// entity references with the `push` method.
///
/// There should only be a single `PrimaryMap` instance for a given `EntityRef` type, otherwise
/// conflicting references will be created. Using unknown keys for indexing will cause a panic.
#[derive(Debug, Clone)]
pub struct PrimaryMap<K, V>
    where K: EntityRef
{
    elems: Vec<V>,
    unused: PhantomData<K>,
}

impl<K, V> PrimaryMap<K, V>
    where K: EntityRef
{
    /// Create a new empty map.
    pub fn new() -> Self {
        PrimaryMap {
            elems: Vec::new(),
            unused: PhantomData,
        }
    }

    /// Check if `k` is a valid key in the map.
    pub fn is_valid(&self, k: K) -> bool {
        k.index() < self.elems.len()
    }

    /// Get the element at `k` if it exists.
    pub fn get(&self, k: K) -> Option<&V> {
        self.elems.get(k.index())
    }

    /// Is this map completely empty?
    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    /// Get the total number of entity references created.
    pub fn len(&self) -> usize {
        self.elems.len()
    }

    /// Iterate over all the keys in this map.
    pub fn keys(&self) -> Keys<K> {
        Keys::new(self.elems.len())
    }

    /// Remove all entries from this map.
    pub fn clear(&mut self) {
        self.elems.clear()
    }

    /// Get the key that will be assigned to the next pushed value.
    pub fn next_key(&self) -> K {
        K::new(self.elems.len())
    }

    /// Append `v` to the mapping, assigning a new key which is returned.
    pub fn push(&mut self, v: V) -> K {
        let k = self.next_key();
        self.elems.push(v);
        k
    }
}

/// Immutable indexing into an `PrimaryMap`.
/// The indexed value must be in the map.
impl<K, V> Index<K> for PrimaryMap<K, V>
    where K: EntityRef
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        &self.elems[k.index()]
    }
}

/// Mutable indexing into an `PrimaryMap`.
impl<K, V> IndexMut<K> for PrimaryMap<K, V>
    where K: EntityRef
{
    fn index_mut(&mut self, k: K) -> &mut V {
        &mut self.elems[k.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `EntityRef` impl for testing.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct E(u32);

    impl EntityRef for E {
        fn new(i: usize) -> Self {
            E(i as u32)
        }
        fn index(self) -> usize {
            self.0 as usize
        }
    }

    #[test]
    fn basic() {
        let r0 = E(0);
        let r1 = E(1);
        let m = PrimaryMap::<E, isize>::new();

        let v: Vec<E> = m.keys().collect();
        assert_eq!(v, []);

        assert!(!m.is_valid(r0));
        assert!(!m.is_valid(r1));
    }

    #[test]
    fn push() {
        let mut m = PrimaryMap::new();
        let k1: E = m.push(12);
        let k2 = m.push(33);

        assert_eq!(m[k1], 12);
        assert_eq!(m[k2], 33);

        let v: Vec<E> = m.keys().collect();
        assert_eq!(v, [k1, k2]);
    }
}
