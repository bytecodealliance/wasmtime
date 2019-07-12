//! Densely numbered entity references as mapping keys.

use crate::iter::{Iter, IterMut};
use crate::keys::Keys;
use crate::EntityRef;
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};
use core::slice;
use std::vec::Vec;

/// A mapping `K -> V` for densely indexed entity references.
///
/// The `SecondaryMap` data structure uses the dense index space to implement a map with a vector.
/// Unlike `PrimaryMap`, an `SecondaryMap` can't be used to allocate entity references. It is used
/// to associate secondary information with entities.
///
/// The map does not track if an entry for a key has been inserted or not. Instead it behaves as if
/// all keys have a default entry from the beginning.
#[derive(Debug, Clone)]
pub struct SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    elems: Vec<V>,
    default: V,
    unused: PhantomData<K>,
}

/// Shared `SecondaryMap` implementation for all value types.
impl<K, V> SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    /// Create a new empty map.
    pub fn new() -> Self
    where
        V: Default,
    {
        Self {
            elems: Vec::new(),
            default: Default::default(),
            unused: PhantomData,
        }
    }

    /// Create a new empty map with a specified default value.
    ///
    /// This constructor does not require V to implement Default.
    pub fn with_default(default: V) -> Self {
        Self {
            elems: Vec::new(),
            default,
            unused: PhantomData,
        }
    }

    /// Returns the number of elements in the underlying vector.
    ///
    /// The number is not necessarily the same as the length of the corresponding PrimaryMap.
    pub fn len(&self) -> usize {
        self.elems.len()
    }

    /// Get the element at `k` if it exists.
    pub fn get(&self, k: K) -> Option<&V> {
        self.elems.get(k.index())
    }

    /// Get the default value.
    pub fn get_default(&self) -> &V {
        &self.default
    }

    /// Is this map completely empty?
    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    /// Remove all entries from this map.
    pub fn clear(&mut self) {
        self.elems.clear()
    }

    /// Iterate over all the keys and values in this map.
    pub fn iter(&self) -> Iter<K, V> {
        Iter::new(self.elems.iter())
    }

    /// Iterate over all the keys and values in this map, mutable edition.
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        IterMut::new(self.elems.iter_mut())
    }

    /// Iterate over all the keys in this map.
    pub fn keys(&self) -> Keys<K> {
        Keys::with_len(self.elems.len())
    }

    /// Iterate over all the values in this map.
    pub fn values(&self) -> slice::Iter<V> {
        self.elems.iter()
    }

    /// Iterate over all the values in this map, mutable edition.
    pub fn values_mut(&mut self) -> slice::IterMut<V> {
        self.elems.iter_mut()
    }

    /// Resize the map to have `n` entries by adding default entries as needed.
    #[inline]
    pub fn resize(&mut self, n: usize) {
        self.elems.resize(n, self.default.clone());
    }
}

/// Immutable indexing into an `SecondaryMap`.
///
/// All keys are permitted. Untouched entries have the default value.
impl<K, V> Index<K> for SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        self.get(k).unwrap_or(&self.default)
    }
}

/// Mutable indexing into an `SecondaryMap`.
///
/// The map grows as needed to accommodate new keys.
impl<K, V> IndexMut<K> for SecondaryMap<K, V>
where
    K: EntityRef,
    V: Clone,
{
    #[inline]
    fn index_mut(&mut self, k: K) -> &mut V {
        let i = k.index();
        if i >= self.elems.len() {
            self.resize(i + 1);
        }
        &mut self.elems[i]
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
        let r2 = E(2);
        let mut m = SecondaryMap::new();

        let v: Vec<E> = m.keys().collect();
        assert_eq!(v, []);

        m[r2] = 3;
        m[r1] = 5;

        assert_eq!(m[r1], 5);
        assert_eq!(m[r2], 3);

        let v: Vec<E> = m.keys().collect();
        assert_eq!(v, [r0, r1, r2]);

        let shared = &m;
        assert_eq!(shared[r0], 0);
        assert_eq!(shared[r1], 5);
        assert_eq!(shared[r2], 3);
    }
}
