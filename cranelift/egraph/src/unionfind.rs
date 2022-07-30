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
    pub fn new() -> Self {
        UnionFind { parent: vec![] }
    }

    pub fn with_capacity(cap: usize) -> Self {
        UnionFind {
            parent: Vec::with_capacity(cap),
        }
    }

    pub fn add(&mut self, id: Id) {
        if id.index() >= self.parent.len() {
            self.parent.resize(id.index() + 1, Id::invalid());
        }
        self.parent[id.index()] = id;
    }

    pub fn find(&self, mut node: Id) -> Id {
        while node != self.parent[node.index()] {
            node = self.parent[node.index()];
        }
        node
    }

    pub fn find_and_update(&mut self, mut node: Id) -> Id {
        // "Path splitting" mutating find (Tarjan and Van Leeuwen).
        while node != self.parent[node.index()] {
            let next = self.parent[self.parent[node.index()].index()];
            self.parent[node.index()] = next;
            node = next;
        }
        node
    }

    pub fn union(&mut self, a: Id, b: Id) {
        let a = self.find_and_update(a);
        let b = self.find_and_update(b);
        if a != b {
            self.parent[b.index()] = a;
        }
    }
}
