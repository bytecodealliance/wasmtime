//! Densely numbered entity references as mapping keys.
//!
//! This module defines an `EntityRef` trait that should be implemented by reference types wrapping
//! a small integer index. The `EntityMap` data structure uses the dense index space to implement a
//! map with a vector. There are primary and secondary entity maps:
//!
//! - A *primary* `EntityMap` contains the main definition of an entity, and it can be used to
//!   allocate new entity references with the `push` method. The values stores in a primary map
//!   must implement the `PrimaryEntityData` marker trait.
//! - A *secondary* `EntityMap` contains additional data about entities kept in a primary map. The
//!   values need to implement `Clone + Default` traits so the map can be grown with `ensure`.

use std::vec::Vec;
use std::default::Default;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// A type wrapping a small integer index should implement `EntityRef` so it can be used as the key
/// of an `EntityMap`.
pub trait EntityRef: Copy + Eq {
    /// Create a new entity reference from a small integer.
    /// This should crash if the requested index is not representable.
    fn new(usize) -> Self;

    /// Get the index that was used to create this entity reference.
    fn index(self) -> usize;
}

/// A mapping `K -> V` for densely indexed entity references.
#[derive(Debug, Clone)]
pub struct EntityMap<K, V>
    where K: EntityRef
{
    elems: Vec<V>,
    unused: PhantomData<K>,
}

/// Shared `EntityMap` implementation for all value types.
impl<K, V> EntityMap<K, V>
    where K: EntityRef
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

    /// Get the element at `k` if it exists.
    pub fn get(&self, k: K) -> Option<&V> {
        self.elems.get(k.index())
    }

    /// Is this map completely empty?
    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    /// Remove all entries from this map.
    pub fn clear(&mut self) {
        self.elems.clear()
    }

    /// Iterate over all the keys in this map.
    pub fn keys(&self) -> Keys<K> {
        Keys {
            pos: 0,
            rev_pos: self.elems.len(),
            unused: PhantomData,
        }
    }
}

/// A marker trait for data stored in primary entity maps.
///
/// A primary entity map can be used to allocate new entity references with the `push` method. It
/// is important that entity references can't be created anywhere else, so the data stored in a
/// primary entity map must be tagged as `PrimaryEntityData` to unlock the `push` method.
pub trait PrimaryEntityData {}

/// Additional methods for primary entry maps only.
///
/// These are identified by the `PrimaryEntityData` marker trait.
impl<K, V> EntityMap<K, V>
    where K: EntityRef,
          V: PrimaryEntityData
{
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

    /// Get the total number of entity references created.
    pub fn len(&self) -> usize {
        self.elems.len()
    }
}

/// Additional methods for value types that implement `Clone` and `Default`.
///
/// When the value type implements these additional traits, the `EntityMap` can be resized
/// explicitly with the `ensure` method.
///
/// Use this for secondary maps that are mapping keys created by another primary map.
impl<K, V> EntityMap<K, V>
    where K: EntityRef,
          V: Clone + Default
{
    /// Create a new secondary `EntityMap` that is prepared to hold `n` elements.
    ///
    /// Use this when the length of the primary map is known:
    /// ```
    /// let secondary_map = EntityMap::with_capacity(primary_map.len());
    /// ```
    pub fn with_capacity(n: usize) -> Self {
        let mut map = EntityMap {
            elems: Vec::with_capacity(n),
            unused: PhantomData,
        };
        map.elems.resize(n, V::default());
        map
    }

    /// Resize the map to have `n` entries by adding default entries as needed.
    pub fn resize(&mut self, n: usize) {
        self.elems.resize(n, V::default());
    }

    /// Ensure that `k` is a valid key but adding default entries if necessary.
    ///
    /// Return a mutable reference to the corresponding entry.
    pub fn ensure(&mut self, k: K) -> &mut V {
        if !self.is_valid(k) {
            self.resize(k.index() + 1)
        }
        &mut self.elems[k.index()]
    }

    /// Get the element at `k` or the default value if `k` is out of range.
    pub fn get_or_default(&self, k: K) -> V {
        self.elems.get(k.index()).cloned().unwrap_or_default()
    }
}

/// Immutable indexing into an `EntityMap`.
/// The indexed value must be in the map, either because it was created by `push`, or the key was
/// passed to `ensure`.
impl<K, V> Index<K> for EntityMap<K, V>
    where K: EntityRef
{
    type Output = V;

    fn index(&self, k: K) -> &V {
        &self.elems[k.index()]
    }
}

/// Mutable indexing into an `EntityMap`.
/// Use `ensure` instead if the key is not known to be valid.
impl<K, V> IndexMut<K> for EntityMap<K, V>
    where K: EntityRef
{
    fn index_mut(&mut self, k: K) -> &mut V {
        &mut self.elems[k.index()]
    }
}

/// Iterate over all keys in order.
pub struct Keys<K>
    where K: EntityRef
{
    pos: usize,
    rev_pos: usize,
    unused: PhantomData<K>,
}

impl<K> Iterator for Keys<K>
    where K: EntityRef
{
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.rev_pos {
            let k = K::new(self.pos);
            self.pos += 1;
            Some(k)
        } else {
            None
        }
    }
}

impl<K> DoubleEndedIterator for Keys<K>
    where K: EntityRef
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.rev_pos > self.pos {
            let k = K::new(self.rev_pos - 1);
            self.rev_pos -= 1;
            Some(k)
        } else {
            None
        }
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

    impl PrimaryEntityData for isize {}

    #[test]
    fn basic() {
        let r0 = E(0);
        let r1 = E(1);
        let r2 = E(2);
        let mut m = EntityMap::new();

        let v: Vec<E> = m.keys().collect();
        assert_eq!(v, []);

        assert!(!m.is_valid(r0));
        m.ensure(r2);
        m[r2] = 3;
        assert!(m.is_valid(r1));
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

    #[test]
    fn push() {
        let mut m = EntityMap::new();
        let k1: E = m.push(12);
        let k2 = m.push(33);

        assert_eq!(m[k1], 12);
        assert_eq!(m[k2], 33);
    }
}
