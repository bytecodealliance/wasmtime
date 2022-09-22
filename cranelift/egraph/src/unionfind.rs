//! Simple union-find data structure.

use crate::{trace, Id};
use cranelift_entity::SecondaryMap;

/// A union-find data structure. The data structure can allocate
/// `Id`s, indicating eclasses, and can merge eclasses together.
#[derive(Clone, Debug)]
pub struct UnionFind {
    parent: SecondaryMap<Id, Id>,
}

impl UnionFind {
    /// Create a new `UnionFind`.
    pub fn new() -> Self {
        UnionFind {
            parent: SecondaryMap::new(),
        }
    }

    /// Create a new `UnionFind` with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        UnionFind {
            parent: SecondaryMap::with_capacity(cap),
        }
    }

    /// Add an `Id` to the `UnionFind`, with its own equivalence class
    /// initially. All `Id`s must be added before being queried or
    /// unioned.
    pub fn add(&mut self, id: Id) {
        self.parent[id] = id;
    }

    /// Find the canonical `Id` of a given `Id`.
    pub fn find(&self, mut node: Id) -> Id {
        while node != self.parent[node] {
            node = self.parent[node];
        }
        node
    }

    /// Find the canonical `Id` of a given `Id`, updating the data
    /// structure in the process so that future queries for this `Id`
    /// (and others in its chain up to the root of the equivalence
    /// class) will be faster.
    pub fn find_and_update(&mut self, mut node: Id) -> Id {
        // "Path splitting" mutating find (Tarjan and Van Leeuwen).
        let orig = node;
        while node != self.parent[node] {
            let next = self.parent[self.parent[node]];
            self.parent[node] = next;
            node = next;
        }
        trace!("find_and_update: {} -> {}", orig, node);
        node
    }

    /// Merge the equivalence classes of the two `Id`s.
    pub fn union(&mut self, a: Id, b: Id) {
        let a = self.find_and_update(a);
        let b = self.find_and_update(b);
        let (a, b) = (std::cmp::min(a, b), std::cmp::max(a, b));
        if a != b {
            // Always canonicalize toward lower IDs.
            self.parent[b] = a;
            trace!("union: {}, {}", a, b);
        }
    }
}
