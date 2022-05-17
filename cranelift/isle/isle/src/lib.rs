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
#[derive(Clone, Debug)]
struct StableSet<T>(HashSet<T>);

impl<T> StableSet<T> {
    fn new() -> Self {
        StableSet(HashSet::new())
    }
}

impl<T: Hash + Eq> StableSet<T> {
    fn insert(&mut self, val: T) -> bool {
        self.0.insert(val)
    }

    fn contains(&self, val: &T) -> bool {
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

pub mod ast;
pub mod codegen;
pub mod compile;
pub mod error;
pub mod ir;
pub mod lexer;
mod log;
pub mod parser;
pub mod sema;
pub mod trie;

#[cfg(feature = "miette-errors")]
mod error_miette;
