//! Densely numbered entity references as set keys.

use crate::keys::Keys;
use crate::EntityRef;
use core::marker::PhantomData;
use cranelift_bitset::CompoundBitSet;

/// A set of `K` for densely indexed entity references.
///
/// The `EntitySet` data structure uses the dense index space to implement a set with a bitvector.
/// Like `SecondaryMap`, an `EntitySet` is used to associate secondary information with entities.
#[derive(Debug, Clone)]
pub struct EntitySet<K>
where
    K: EntityRef,
{
    bitset: CompoundBitSet,
    unused: PhantomData<K>,
}

impl<K: EntityRef> Default for EntitySet<K> {
    fn default() -> Self {
        Self {
            bitset: CompoundBitSet::default(),
            unused: PhantomData,
        }
    }
}

/// Shared `EntitySet` implementation for all value types.
impl<K> EntitySet<K>
where
    K: EntityRef,
{
    /// Create a new empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new empty set with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bitset: CompoundBitSet::with_capacity(capacity),
            unused: PhantomData,
        }
    }

    /// Get the element at `k` if it exists.
    pub fn contains(&self, k: K) -> bool {
        let index = k.index();
        self.bitset.contains(index)
    }

    /// Is this set completely empty?
    pub fn is_empty(&self) -> bool {
        self.bitset.is_empty()
    }

    /// Remove all entries from this set.
    pub fn clear(&mut self) {
        self.bitset.clear()
    }

    /// Iterate over all the keys in this set.
    pub fn keys(&self) -> Keys<K> {
        Keys::with_len(self.bitset.max().map_or(0, |x| x + 1))
    }

    /// Insert the element at `k`.
    pub fn insert(&mut self, k: K) -> bool {
        let index = k.index();
        self.bitset.insert(index)
    }

    /// Removes and returns the entity from the set if it exists.
    pub fn pop(&mut self) -> Option<K> {
        let index = self.bitset.pop()?;
        Some(K::new(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use core::u32;

    // `EntityRef` impl for testing.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
        let mut m = EntitySet::new();

        let v: Vec<E> = m.keys().collect();
        assert_eq!(v, []);
        assert!(m.is_empty());

        m.insert(r2);
        m.insert(r1);

        assert!(!m.contains(r0));
        assert!(m.contains(r1));
        assert!(m.contains(r2));
        assert!(!m.contains(E(3)));
        assert!(!m.is_empty());

        let v: Vec<E> = m.keys().collect();
        assert_eq!(v, [r0, r1, r2]);

        assert!(!m.contains(E(3)));
        assert!(!m.contains(E(4)));
        assert!(!m.contains(E(8)));
        assert!(!m.contains(E(15)));
        assert!(!m.contains(E(19)));

        m.insert(E(8));
        m.insert(E(15));
        assert!(!m.contains(E(3)));
        assert!(!m.contains(E(4)));
        assert!(m.contains(E(8)));
        assert!(!m.contains(E(9)));
        assert!(!m.contains(E(14)));
        assert!(m.contains(E(15)));
        assert!(!m.contains(E(16)));
        assert!(!m.contains(E(19)));
        assert!(!m.contains(E(20)));
        assert!(!m.contains(E(u32::MAX)));

        m.clear();
        assert!(m.is_empty());
    }

    #[test]
    fn pop_ordered() {
        let r0 = E(0);
        let r1 = E(1);
        let r2 = E(2);
        let mut m = EntitySet::new();
        m.insert(r0);
        m.insert(r1);
        m.insert(r2);

        assert_eq!(r2, m.pop().unwrap());
        assert_eq!(r1, m.pop().unwrap());
        assert_eq!(r0, m.pop().unwrap());
        assert!(m.pop().is_none());
        assert!(m.pop().is_none());
    }

    #[test]
    fn pop_unordered() {
        let mut blocks = [
            E(0),
            E(1),
            E(6),
            E(7),
            E(5),
            E(9),
            E(10),
            E(2),
            E(3),
            E(11),
            E(12),
        ];

        let mut m = EntitySet::new();
        for &block in &blocks {
            m.insert(block);
        }
        assert_eq!(m.bitset.max(), Some(12));
        blocks.sort();

        for &block in blocks.iter().rev() {
            assert_eq!(block, m.pop().unwrap());
        }

        assert!(m.is_empty());
    }
}
