//! Densely numbered entity references as set keys.

use std::marker::PhantomData;
use std::vec::Vec;
use {EntityRef, Keys};

/// A set of `K` for densely indexed entity references.
///
/// The `EntitySet` data structure uses the dense index space to implement a set with a bitvector.
/// Like `SecondaryMap`, an `EntitySet` is used to associate secondary information with entities.
#[derive(Debug, Clone)]
pub struct EntitySet<K>
where
    K: EntityRef,
{
    elems: Vec<u8>,
    len: usize,
    unused: PhantomData<K>,
}

/// Shared `EntitySet` implementation for all value types.
impl<K> EntitySet<K>
where
    K: EntityRef,
{
    /// Create a new empty set.
    pub fn new() -> Self {
        Self {
            elems: Vec::new(),
            len: 0,
            unused: PhantomData,
        }
    }

    /// Get the element at `k` if it exists.
    pub fn contains(&self, k: K) -> bool {
        let index = k.index();
        if index < self.len {
            (self.elems[index / 8] & (1 << (index % 8))) != 0
        } else {
            false
        }
    }

    /// Is this set completely empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Remove all entries from this set.
    pub fn clear(&mut self) {
        self.len = 0;
        self.elems.clear()
    }

    /// Iterate over all the keys in this set.
    pub fn keys(&self) -> Keys<K> {
        Keys::with_len(self.len)
    }

    /// Resize the set to have `n` entries by adding default entries as needed.
    pub fn resize(&mut self, n: usize) {
        self.elems.resize((n + 7) / 8, 0);
        self.len = n
    }

    /// Insert the element at `k`.
    pub fn insert(&mut self, k: K) -> bool {
        let index = k.index();
        if index >= self.len {
            self.resize(index + 1)
        }
        let result = !self.contains(k);
        self.elems[index / 8] |= 1 << (index % 8);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::u32;

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

        m.resize(20);
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
}
