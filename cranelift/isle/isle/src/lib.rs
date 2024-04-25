#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ops::Index;

macro_rules! declare_id {
    (
        $(#[$attr:meta])*
            $name:ident
    ) => {
        $(#[$attr])*
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub usize);
        impl $name {
            /// Get the index of this id.
            pub fn index(self) -> usize {
                self.0
            }
        }
    };
}

/// A wrapper around a [HashSet] which prevents accidentally observing the non-deterministic
/// iteration order.
#[derive(Clone, Debug, Default)]
pub struct StableSet<T>(HashSet<T>);

impl<T> StableSet<T> {
    fn new() -> Self {
        StableSet(HashSet::new())
    }
}

impl<T: Hash + Eq> StableSet<T> {
    /// Adds a value to the set. Returns whether the value was newly inserted.
    pub fn insert(&mut self, val: T) -> bool {
        self.0.insert(val)
    }

    /// Returns true if the set contains a value.
    pub fn contains(&self, val: &T) -> bool {
        self.0.contains(val)
    }
}

/// A wrapper around a [HashMap] which prevents accidentally observing the non-deterministic
/// iteration order.
#[derive(Clone, Debug)]
pub struct StableMap<K, V>(HashMap<K, V>);

impl<K, V> StableMap<K, V> {
    fn new() -> Self {
        StableMap(HashMap::new())
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

// NOTE: Can't auto-derive this
impl<K, V> Default for StableMap<K, V> {
    fn default() -> Self {
        StableMap(HashMap::new())
    }
}

impl<K: Hash + Eq, V> StableMap<K, V> {
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.0.insert(k, v)
    }

    fn contains_key(&self, k: &K) -> bool {
        self.0.contains_key(k)
    }

    fn get(&self, k: &K) -> Option<&V> {
        self.0.get(k)
    }

    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.0.get_mut(k)
    }

    fn remove(&mut self, k: &K) -> Option<V> {
        self.0.remove(k)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn entry(&mut self, k: K) -> Entry<K, V> {
        self.0.entry(k)
    }
}

impl<K: Hash + Eq, V> Index<&K> for StableMap<K, V> {
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.0.index(index)
    }
}

/// Stores disjoint sets and provides efficient operations to merge two sets, and to find a
/// representative member of a set given any member of that set. In this implementation, sets always
/// have at least two members, and can only be formed by the `merge` operation.
#[derive(Clone, Debug, Default)]
pub struct DisjointSets<T> {
    parent: StableMap<T, (T, u8)>,
}

impl<T: Copy + std::fmt::Debug + Eq + Hash> DisjointSets<T> {
    /// Find a representative member of the set containing `x`. If `x` has not been merged with any
    /// other items using `merge`, returns `None`. This method updates the data structure to make
    /// future queries faster, and takes amortized constant time.
    ///
    /// ```
    /// let mut sets = cranelift_isle::DisjointSets::default();
    /// sets.merge(1, 2);
    /// sets.merge(1, 3);
    /// sets.merge(2, 4);
    /// assert_eq!(sets.find_mut(3).unwrap(), sets.find_mut(4).unwrap());
    /// assert_eq!(sets.find_mut(10), None);
    /// ```
    pub fn find_mut(&mut self, mut x: T) -> Option<T> {
        while let Some(node) = self.parent.get(&x) {
            if node.0 == x {
                return Some(x);
            }
            let grandparent = self.parent[&node.0].0;
            // Re-do the lookup but take a mutable borrow this time
            self.parent.get_mut(&x).unwrap().0 = grandparent;
            x = grandparent;
        }
        None
    }

    /// Find a representative member of the set containing `x`. If `x` has not been merged with any
    /// other items using `merge`, returns `None`. This method does not update the data structure to
    /// make future queries faster, so `find_mut` should be preferred.
    ///
    /// ```
    /// let mut sets = cranelift_isle::DisjointSets::default();
    /// sets.merge(1, 2);
    /// sets.merge(1, 3);
    /// sets.merge(2, 4);
    /// assert_eq!(sets.find(3).unwrap(), sets.find(4).unwrap());
    /// assert_eq!(sets.find(10), None);
    /// ```
    pub fn find(&self, mut x: T) -> Option<T> {
        while let Some(node) = self.parent.get(&x) {
            if node.0 == x {
                return Some(x);
            }
            x = node.0;
        }
        None
    }

    /// Merge the set containing `x` with the set containing `y`. This method takes amortized
    /// constant time.
    pub fn merge(&mut self, x: T, y: T) {
        assert_ne!(x, y);
        let mut x = if let Some(x) = self.find_mut(x) {
            self.parent[&x]
        } else {
            self.parent.insert(x, (x, 0));
            (x, 0)
        };
        let mut y = if let Some(y) = self.find_mut(y) {
            self.parent[&y]
        } else {
            self.parent.insert(y, (y, 0));
            (y, 0)
        };

        if x == y {
            return;
        }

        if x.1 < y.1 {
            std::mem::swap(&mut x, &mut y);
        }

        self.parent.get_mut(&y.0).unwrap().0 = x.0;
        if x.1 == y.1 {
            let x_rank = &mut self.parent.get_mut(&x.0).unwrap().1;
            *x_rank = x_rank.saturating_add(1);
        }
    }

    /// Returns whether the given items have both been merged into the same set. If either is not
    /// part of any set, returns `false`.
    ///
    /// ```
    /// let mut sets = cranelift_isle::DisjointSets::default();
    /// sets.merge(1, 2);
    /// sets.merge(1, 3);
    /// sets.merge(2, 4);
    /// sets.merge(5, 6);
    /// assert!(sets.in_same_set(2, 3));
    /// assert!(sets.in_same_set(1, 4));
    /// assert!(sets.in_same_set(3, 4));
    /// assert!(!sets.in_same_set(4, 5));
    /// ```
    pub fn in_same_set(&self, x: T, y: T) -> bool {
        let x = self.find(x);
        let y = self.find(y);
        x.zip(y).filter(|(x, y)| x == y).is_some()
    }

    /// Remove the set containing the given item, and return all members of that set. The set is
    /// returned in sorted order. This method takes time linear in the total size of all sets.
    ///
    /// ```
    /// let mut sets = cranelift_isle::DisjointSets::default();
    /// sets.merge(1, 2);
    /// sets.merge(1, 3);
    /// sets.merge(2, 4);
    /// assert_eq!(sets.remove_set_of(4), &[1, 2, 3, 4]);
    /// assert_eq!(sets.remove_set_of(1), &[]);
    /// assert!(sets.is_empty());
    /// ```
    pub fn remove_set_of(&mut self, x: T) -> Vec<T>
    where
        T: Ord,
    {
        let mut set = Vec::new();
        if let Some(x) = self.find_mut(x) {
            set.extend(self.parent.0.keys().copied());
            // It's important to use `find_mut` here to avoid quadratic worst-case time.
            set.retain(|&y| self.find_mut(y).unwrap() == x);
            for y in set.iter() {
                self.parent.remove(y);
            }
            set.sort_unstable();
        }
        set
    }

    /// Returns true if there are no sets. This method takes constant time.
    ///
    /// ```
    /// let mut sets = cranelift_isle::DisjointSets::default();
    /// assert!(sets.is_empty());
    /// sets.merge(1, 2);
    /// assert!(!sets.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.parent.is_empty()
    }

    /// Returns the total number of elements in all sets. This method takes constant time.
    ///
    /// ```
    /// let mut sets = cranelift_isle::DisjointSets::default();
    /// sets.merge(1, 2);
    /// assert_eq!(sets.len(), 2);
    /// sets.merge(3, 4);
    /// sets.merge(3, 5);
    /// assert_eq!(sets.len(), 5);
    /// ```
    pub fn len(&self) -> usize {
        self.parent.len()
    }
}

pub mod ast;
pub mod codegen;
pub mod compile;
pub mod error;
pub mod lexer;
mod log;
pub mod overlap;
pub mod parser;
pub mod sema;
pub mod serialize;
pub mod trie_again;
