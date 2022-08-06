//! Egraph implementation.

use crate::ctxhash::{CtxEq, CtxHash, CtxHashMap, Entry};
use crate::unionfind::UnionFind;
use crate::{Id, Language};
use cranelift_entity::PrimaryMap;
use smallvec::{smallvec, SmallVec};
use std::marker::PhantomData;

/// An egraph.
pub struct EGraph<L: Language> {
    /// Node-allocation arena.
    pub nodes: Vec<L::Node>,
    /// Hash-consing map from Nodes to eclass IDs.
    node_map: CtxHashMap<NodeKey, Id>,
    /// Eclass definitions. Each eclass consists of an enode, and
    /// parent pointer to the rest of the eclass.
    pub classes: PrimaryMap<Id, EClass>,
    /// Union-find for canonical ID generation. This lets us name an
    /// eclass with a canonical ID that is the same for all
    /// generations of the class.
    pub unionfind: UnionFind,
}

/// A reference to a node.
#[derive(Clone, Copy, Debug)]
pub struct NodeKey {
    index: u32,
}

impl NodeKey {
    fn from_node_idx(node_idx: usize) -> NodeKey {
        NodeKey {
            index: u32::try_from(node_idx).unwrap(),
        }
    }

    /// Get the node for this NodeKey, given the `nodes` from the
    /// appropriate `EGraph`.
    pub fn node<'a, L: Language>(&self, nodes: &'a [L::Node]) -> &'a L::Node {
        &nodes[self.index as usize]
    }

    fn bits(self) -> u32 {
        self.index
    }

    fn from_bits(bits: u32) -> Self {
        NodeKey { index: bits }
    }
}

struct NodeKeyCtx<'a, L: Language> {
    nodes: &'a [L::Node],
    node_ctx: &'a mut L,
}

impl<'ctx, L: Language> CtxEq<NodeKey, NodeKey> for NodeKeyCtx<'ctx, L> {
    fn ctx_eq(&self, a: &NodeKey, b: &NodeKey) -> bool {
        let a = a.node::<L>(self.nodes);
        let b = b.node::<L>(self.nodes);
        self.node_ctx.ctx_eq(a, b)
    }
}

impl<'ctx, L: Language> CtxHash<NodeKey> for NodeKeyCtx<'ctx, L> {
    fn ctx_hash<H: std::hash::Hasher>(&self, value: &NodeKey, state: &mut H) {
        self.node_ctx.ctx_hash(value.node::<L>(self.nodes), state);
    }
}

/// An EClass entry. Contains either a single new enode and a parent
/// eclass (i.e., adds one new enode), or unions two parent eclasses
/// together.
#[derive(Debug, Clone, Copy)]
pub struct EClass {
    // formats:
    //
    // 00 | unused  (31 bits)         | NodeKey (31 bits)
    // 01 | eclass_parent   (31 bits) | NodeKey (31 bits)
    // 10 | eclass_parent_1 (31 bits) | eclass_parent_id_2 (31 bits)
    bits: u64,
}

impl EClass {
    fn node(node: NodeKey) -> EClass {
        let node_idx = node.bits() as u64;
        debug_assert!(node_idx < (1 << 31));
        EClass {
            bits: (0b00 << 62) | node_idx,
        }
    }

    fn node_and_parent(node: NodeKey, eclass_parent: Id) -> EClass {
        let node_idx = node.bits() as u64;
        debug_assert!(node_idx < (1 << 31));
        debug_assert!(eclass_parent != Id::invalid());
        let parent = eclass_parent.0 as u64;
        debug_assert!(parent < (1 << 31));
        EClass {
            bits: (0b01 << 62) | (parent << 31) | node_idx,
        }
    }

    fn union(parent1: Id, parent2: Id) -> EClass {
        debug_assert!(parent1 != Id::invalid());
        let parent1 = parent1.0 as u64;
        debug_assert!(parent1 < (1 << 31));

        debug_assert!(parent2 != Id::invalid());
        let parent2 = parent2.0 as u64;
        debug_assert!(parent2 < (1 << 31));

        EClass {
            bits: (0b10 << 62) | (parent1 << 31) | parent2,
        }
    }

    /// Get the node, if any, from a node-only or node-and-parent
    /// eclass.
    pub fn get_node(&self) -> Option<NodeKey> {
        self.as_node()
            .or_else(|| self.as_node_and_parent().map(|(node, _)| node))
    }

    /// If this EClass is just a lone enode, return it.
    pub fn as_node(&self) -> Option<NodeKey> {
        if (self.bits >> 62) == 0b00 {
            let node_idx = (self.bits & ((1 << 31) - 1)) as u32;
            Some(NodeKey::from_bits(node_idx))
        } else {
            None
        }
    }

    /// If this EClass is one new enode and a parent, return the node
    /// and parent ID.
    pub fn as_node_and_parent(&self) -> Option<(NodeKey, Id)> {
        if (self.bits >> 62) == 0b01 {
            let node_idx = (self.bits & ((1 << 31) - 1)) as u32;
            let parent = ((self.bits >> 31) & ((1 << 31) - 1)) as u32;
            Some((NodeKey::from_bits(node_idx), Id::from_bits(parent)))
        } else {
            None
        }
    }

    /// If this EClass is the union variety, return the two parent
    /// EClasses. Both are guaranteed not to be `Id::invalid()`.
    pub fn as_union(&self) -> Option<(Id, Id)> {
        if (self.bits >> 62) == 0b10 {
            let parent1 = ((self.bits >> 31) & ((1 << 31) - 1)) as u32;
            let parent2 = (self.bits & ((1 << 31) - 1)) as u32;
            Some((Id::from_bits(parent1), Id::from_bits(parent2)))
        } else {
            None
        }
    }
}

/// A new or existing `T` when adding to a deduplicated set or data
/// structure, like an egraph.
#[derive(Clone, Copy, Debug)]
pub enum NewOrExisting<T> {
    New(T),
    Existing(T),
}

impl<T> NewOrExisting<T> {
    /// Get the underlying value.
    pub fn get(self) -> T {
        match self {
            NewOrExisting::New(t) => t,
            NewOrExisting::Existing(t) => t,
        }
    }
}

impl<L: Language> EGraph<L>
where
    L::Node: 'static,
{
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            node_map: CtxHashMap::new(),
            classes: PrimaryMap::new(),
            unionfind: UnionFind::new(),
        }
    }

    pub fn with_capacity(nodes: usize) -> Self {
        Self {
            nodes: vec![],
            node_map: CtxHashMap::with_capacity(nodes),
            classes: PrimaryMap::with_capacity(nodes),
            unionfind: UnionFind::with_capacity(nodes),
        }
    }

    /// Add a new node.
    pub fn add(&mut self, node: L::Node, node_ctx: &mut L) -> NewOrExisting<Id> {
        // Push the node. We can then build a NodeKey that refers to
        // it and look for an existing interned copy. If one exists,
        // we can pop the pushed node and return the existing Id.
        let node_idx = self.nodes.len();
        log::trace!("adding node: {:?}", node);
        self.nodes.push(node);

        let key = NodeKey::from_node_idx(node_idx);
        let ctx = NodeKeyCtx {
            nodes: &self.nodes[..],
            node_ctx,
        };

        match self.node_map.entry(key, &ctx) {
            Entry::Occupied(o) => {
                let eclass_id = *o.get();
                self.nodes.pop();
                log::trace!(" -> existing id {}", eclass_id);
                NewOrExisting::Existing(eclass_id)
            }
            Entry::Vacant(v) => {
                // We're creating a new eclass now.
                let eclass_id = self.classes.push(EClass::node(key));
                log::trace!(" -> new node and eclass: {}", eclass_id);
                self.unionfind.add(eclass_id);

                // Add to interning map with a NodeKey referring to the eclass.
                v.insert(eclass_id);

                NewOrExisting::New(eclass_id)
            }
        }
    }

    /// Merge one eclass into another, maintaining the acyclic
    /// property (args must have lower eclass Ids than the eclass
    /// containing the node with those args). Returns the Id of the
    /// merged eclass.
    pub fn union(&mut self, a: Id, b: Id) -> Id {
        assert_ne!(a, Id::invalid());
        assert_ne!(b, Id::invalid());
        let (a, b) = (std::cmp::max(a, b), std::cmp::min(a, b));
        log::trace!("union: id {} and id {}", a, b);
        if a == b {
            log::trace!(" -> no-op");
            return a;
        }

        self.unionfind.union(a, b);

        // If the younger eclass has no parent, we can link it
        // directly and return that eclass. Otherwise, we create a new
        // union eclass.
        if let Some(node) = self.classes[a].as_node() {
            log::trace!(
                " -> id {} is one-node eclass; making into node-and-parent with id {}",
                a,
                b
            );
            self.classes[a] = EClass::node_and_parent(node, b);
            return a;
        }

        let u = self.classes.push(EClass::union(a, b));
        self.unionfind.add(u);
        self.unionfind.union(u, b);
        log::trace!(" -> union id {} and id {} into id {}", a, b, u);
        u
    }

    /// Get the canonical ID for an eclass. This may be an older
    /// generation, so will not be able to see all enodes in the
    /// eclass; but it will allow us to unambiguously refer to an
    /// eclass, even across merging.
    pub fn canonical_id_mut(&mut self, eclass: Id) -> Id {
        self.unionfind.find_and_update(eclass)
    }

    /// Get the canonical ID for an eclass. This may be an older
    /// generation, so will not be able to see all enodes in the
    /// eclass; but it will allow us to unambiguously refer to an
    /// eclass, even across merging.
    pub fn canonical_id(&self, eclass: Id) -> Id {
        self.unionfind.find(eclass)
    }

    /// Get the enodes for a given eclass.
    pub fn enodes(&self, eclass: Id) -> NodeIter<L> {
        NodeIter {
            stack: smallvec![eclass],
            _phantom: PhantomData,
        }
    }
}

/// An iterator over all nodes in an eclass.
///
/// Because eclasses are immutable once created, this does *not* need
/// to hold an open borrow on the egraph; it is free to add new nodes,
/// while our existing Ids will remain valid.
pub struct NodeIter<L: Language> {
    stack: SmallVec<[Id; 8]>,
    _phantom: PhantomData<L>,
}

impl<L: Language> NodeIter<L> {
    pub fn next<'a>(&mut self, egraph: &'a EGraph<L>) -> Option<&'a L::Node> {
        while let Some(next) = self.stack.pop() {
            let eclass = egraph.classes[next];
            if let Some(node) = eclass.as_node() {
                return Some(&egraph.nodes[node.index as usize]);
            } else if let Some((node, parent)) = eclass.as_node_and_parent() {
                if parent != Id::invalid() {
                    self.stack.push(parent);
                }
                return Some(&egraph.nodes[node.index as usize]);
            } else if let Some((parent1, parent2)) = eclass.as_union() {
                debug_assert!(parent1 != Id::invalid());
                debug_assert!(parent2 != Id::invalid());
                self.stack.push(parent2);
                self.stack.push(parent1);
                continue;
            } else {
                unreachable!("Invalid eclass format");
            }
        }
        None
    }
}
