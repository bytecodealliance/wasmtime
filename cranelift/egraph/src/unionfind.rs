//! Simple union-find data structure.

use crate::Id;
use cranelift_entity::EntityRef;

/// A union-find data structure. The data structure can allocate
/// `Id`s, indicating eclasses, and can merge eclasses together.
#[derive(Clone, Debug)]
pub struct UnionFind {
    parent: Vec<Id>,
}

impl UnionFind {
    /// Create a new `UnionFind`.
    pub fn new() -> Self {
        UnionFind { parent: vec![] }
    }

    /// Create a new `UnionFind` with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        UnionFind {
            parent: Vec::with_capacity(cap),
        }
    }

    /// Add an `Id` to the `UnionFind`, with its own equivalence class
    /// initially. All `Id`s must be added before being queried or
    /// unioned.
    pub fn add(&mut self, id: Id) {
        if id.index() >= self.parent.len() {
            self.parent.resize(id.index() + 1, Id::invalid());
        }
        self.parent[id.index()] = id;
    }

    /// Find the canonical `Id` of a given `Id`.
    pub fn find(&self, mut node: Id) -> Id {
        while node != self.parent[node.index()] {
            node = self.parent[node.index()];
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
        while node != self.parent[node.index()] {
            let next = self.parent[self.parent[node.index()].index()];
            self.parent[node.index()] = next;
            node = next;
        }
        log::trace!("find_and_update: {} -> {}", orig, node);
        node
    }

    /// Merge the equivalence classes of the two `Id`s.
    pub fn union(&mut self, a: Id, b: Id) {
        let a = self.find_and_update(a);
        let b = self.find_and_update(b);
        let (a, b) = (std::cmp::min(a, b), std::cmp::max(a, b));
        if a != b {
            // Always canonicalize toward lower IDs.
            self.parent[b.index()] = a;
            log::trace!("union: {}, {}", a, b);
        }
    }
}
