//! Densely numbered entity references as mapping keys.
//!
//! This module defines an `EntityRef` trait that should be implemented by reference types wrapping
//! a small integer index. The `EntityMap` data structure uses the dense index space to implement a
//! map with a vector.

use std::vec::Vec;
use std::default::Default;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// A type wrapping a small integer index should implement `EntityRef` so it can be used as the key
/// of an `EntityMap`.
pub trait EntityRef: Copy {
    /// Create a new entity reference from a small integer.
    /// This should crash if the requested index is not representable.
    fn new(usize) -> Self;

    /// Get the index that was used to create this entity reference.
    fn index(self) -> usize;
}

/// A mapping `K -> V` for densely indexed entity references.
pub struct EntityMap<K, V>
    where K: EntityRef,
          V: Clone + Default
{
    elems: Vec<V>,
    unused: PhantomData<K>,
}

impl<K, V> EntityMap<K, V>
    where K: EntityRef,
          V: Clone + Default
{
    /// Create a new empty map.
    pub fn new() -> Self {
        EntityMap {
            elems: Vec::new(),
            unused: PhantomData,
        }
    }

    /// Check if `k` is a valid key in the map.
    pub fn is_valid(&self, k: K) -> bool {
        k.index() < self.elems.len()
    }

    /// Ensure that `k` is a valid key but adding default entries if necesssary.
    pub fn ensure(&mut self, k: K) {
        if !self.is_valid(k) {
            self.elems.resize(k.index() + 1, V::default())
        }
    }

    /// Append `v` to the mapping, assigning a new key which is returned.
    pub fn push(&mut self, v: V) -> K {
        let k = K::new(self.elems.len());
        self.elems.push(v);
        k
    }
}

/// Immutable indexing into an `EntityMap`.
/// The indexed value must have been accessed mutably previously, or the key passed to `ensure()`.
impl<K, V> Index<K> for EntityMap<K, V>
    where K: EntityRef,
          V: Clone + Default
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        &self.elems[k.index()]
    }
}

/// Mutable indexing into an `EntityMap`.
/// The map is resized automatically if the key has not been used before.
impl<K, V> IndexMut<K> for EntityMap<K, V>
    where K: EntityRef,
          V: Clone + Default
{
    fn index_mut(&mut self, k: K) -> &mut V {
        self.ensure(k);
        &mut self.elems[k.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // EntityRef impl for testing.
    #[derive(Clone, Copy)]
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
        let r2 = E(2);
        let mut m = EntityMap::new();

        assert!(!m.is_valid(r0));
        m[r2] = 3;
        assert!(m.is_valid(r1));
        m[r1] = 5;

        assert_eq!(m[r1], 5);
        assert_eq!(m[r2], 3);

        let shared = &m;
        assert_eq!(shared[r0], 0);
        assert_eq!(shared[r1], 5);
        assert_eq!(shared[r2], 3);
    }

    #[test]
    fn push() {
        let mut m = EntityMap::new();
        let k1: E = m.push(12);
        let k2 = m.push(33);

        assert_eq!(m[k1], 12);
        assert_eq!(m[k2], 33);
    }
}
