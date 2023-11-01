//! Simple union-find data structure.

use crate::trace;
use cranelift_entity::{packed_option::ReservedValue, EntityRef, SecondaryMap};
use std::hash::Hash;

/// A union-find data structure. The data structure can allocate
/// `Id`s, indicating eclasses, and can merge eclasses together.
#[derive(Clone, Debug, PartialEq)]
pub struct UnionFind<Idx: EntityRef> {
    parent: SecondaryMap<Idx, Val<Idx>>,
}

#[derive(Clone, Debug, PartialEq)]
struct Val<Idx>(Idx);

impl<Idx: EntityRef + ReservedValue> Default for Val<Idx> {
    fn default() -> Self {
        Self(Idx::reserved_value())
    }
}

impl<Idx: EntityRef + Hash + std::fmt::Display + Ord + ReservedValue> UnionFind<Idx> {
    /// Create a new `UnionFind` with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        UnionFind {
            parent: SecondaryMap::with_capacity(cap),
        }
    }

    /// Add an `Idx` to the `UnionFind`, with its own equivalence class
    /// initially. All `Idx`s must be added before being queried or
    /// unioned.
    pub fn add(&mut self, id: Idx) {
        debug_assert!(id != Idx::reserved_value());
        self.parent[id] = Val(id);
    }

    /// Find the canonical `Idx` of a given `Idx`.
    pub fn find(&self, mut node: Idx) -> Idx {
        while node != self.parent[node].0 {
            node = self.parent[node].0;
        }
        node
    }

    /// Find the canonical `Idx` of a given `Idx`, updating the data
    /// structure in the process so that future queries for this `Idx`
    /// (and others in its chain up to the root of the equivalence
    /// class) will be faster.
    pub fn find_and_update(&mut self, mut node: Idx) -> Idx {
        // "Path halving" mutating find (Tarjan and Van Leeuwen).
        debug_assert!(node != Idx::reserved_value());
        while node != self.parent[node].0 {
            let next = self.parent[self.parent[node].0].0;
            debug_assert!(next != Idx::reserved_value());
            self.parent[node] = Val(next);
            node = next;
        }
        debug_assert!(node != Idx::reserved_value());
        node
    }

    /// Merge the equivalence classes of the two `Idx`s.
    pub fn union(&mut self, a: Idx, b: Idx) {
        let a = self.find_and_update(a);
        let b = self.find_and_update(b);
        let (a, b) = (std::cmp::min(a, b), std::cmp::max(a, b));
        if a != b {
            // Always canonicalize toward lower IDs.
            self.parent[b] = Val(a);
            trace!("union: {}, {}", a, b);
        }
    }
}
