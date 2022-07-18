//! Egraph implementation.

use crate::bumpvec::{BumpArena, BumpVec};
use crate::ctxhash::{CtxEq, CtxHash, CtxHashMap};
use crate::unionfind::UnionFind;
use crate::{Id, Language};
use cranelift_entity::{EntityList, ListPool, PrimaryMap};
use std::marker::PhantomData;

/// An egraph.
pub struct EGraph<L: Language>
where
    L::Node: 'static,
{
    /// Node-allocation arena.
    node_arena: BumpArena<L::Node>,
    /// Hash-consing map from Nodes to eclass IDs.
    node_map: CtxHashMap<NodeKey<'static, L>, Id>,
    /// Union-find data structure representing eclass merging.
    union: UnionFind,
    /// Eclass definitions. Each eclass consists of a list of nodes,
    /// and of parent nodes that refer to this eclass.
    classes: PrimaryMap<Id, EClass<L>>,
    /// List pool used for parent lists.
    parent_pool: ListPool<Id>,
    /// List of node IDs whose children have been updated. These need
    /// to be re-canonicalized in `.rebuild()`.
    pending_child_updates: Vec<Id>,
    /// List of eclass IDs that have been updated (have a new node)
    /// since last rebuild. These need to be processed by any rewrite
    /// rules.
    #[allow(dead_code)]
    pending_dirty_classes: Vec<Id>,
}

/// A reference to a node.
struct NodeKey<'a, L: Language> {
    bits: u64,
    _phantom: PhantomData<&'a L::Node>,
}

impl<'a, L: Language> NodeKey<'a, L>
where
    L::Node: 'static,
{
    fn from_eclass_node(eclass: Id, node_idx: usize) -> NodeKey<'static, L> {
        let node_idx = node_idx & (u32::MAX as usize);
        let bits = ((eclass.as_u32() as u64) << 33) | ((node_idx as u64) << 1) | 1;
        NodeKey {
            bits,
            _phantom: PhantomData,
        }
    }

    fn from_ref(external_node: &'a L::Node) -> NodeKey<'a, L> {
        let bits = external_node as *const L::Node as usize as u64;
        debug_assert_eq!(bits & 1, 0);
        NodeKey {
            bits,
            _phantom: PhantomData,
        }
    }

    fn node<'egraph, 'ret>(
        &'a self,
        classes: &'egraph PrimaryMap<Id, EClass<L>>,
        arena: &'egraph BumpArena<L::Node>,
    ) -> &'ret L::Node
    where
        'a: 'ret,
        'egraph: 'ret,
    {
        if self.bits & 1 != 0 {
            let eclass = Id::from_u32((self.bits >> 33) as u32);
            let node_idx = ((self.bits >> 1) & (u32::MAX as u64)) as usize;
            &classes[eclass].enodes.as_slice(arena)[node_idx]
        } else {
            let borrow: &'a L::Node =
                unsafe { std::mem::transmute(self.bits as usize as *const L::Node) };
            borrow
        }
    }
}

struct NodeKeyCtx<'ctx, L: Language> {
    classes: &'ctx PrimaryMap<Id, EClass<L>>,
    arena: &'ctx BumpArena<L::Node>,
    node_ctx: &'ctx L,
}

impl<'ctx, 'a, 'b, L: Language> CtxEq<NodeKey<'a, L>, NodeKey<'b, L>> for NodeKeyCtx<'ctx, L>
where
    L::Node: 'static,
{
    fn ctx_eq(&self, a: &NodeKey<'a, L>, b: &NodeKey<'b, L>) -> bool {
        let a = a.node(self.classes, self.arena);
        let b = b.node(self.classes, self.arena);
        self.node_ctx.ctx_eq(a, b)
    }
}

impl<'ctx, 'a, L: Language> CtxHash<NodeKey<'a, L>> for NodeKeyCtx<'ctx, L>
where
    L::Node: 'static,
{
    fn ctx_hash<H: std::hash::Hasher>(&self, value: &NodeKey<'a, L>, state: &mut H) {
        self.node_ctx
            .ctx_hash(value.node(self.classes, self.arena), state);
    }
}

#[derive(Debug)]
pub struct EClass<L: Language> {
    enodes: BumpVec<L::Node>,
    parents: EntityList<Id>,
}

impl<L: Language> EGraph<L>
where
    L::Node: 'static,
{
    pub fn new() -> Self {
        Self {
            node_arena: BumpArena::new(),
            node_map: CtxHashMap::new(),
            union: UnionFind::new(),
            classes: PrimaryMap::new(),
            parent_pool: ListPool::new(),
            pending_child_updates: vec![],
            pending_dirty_classes: vec![],
        }
    }

    /// Add a new node.
    pub fn add(&mut self, mut node: L::Node, node_ctx: &mut L) -> Id {
        // Canonicalize all argument eclass IDs. This is relatively
        // cheap if the user has already done so, so we do this
        // unconditionally.
        let num_args = node_ctx.children(&node).len();
        for child in node_ctx.children_mut(&mut node) {
            let id = self.union.find_and_update(*child);
            *child = id;
        }

        // First, intern the node. If it already exists, return the
        // eclass ID for it.
        let key = NodeKey::from_ref(&node);
        let ctx = NodeKeyCtx {
            classes: &self.classes,
            arena: &self.node_arena,
            node_ctx,
        };
        if let Some(eclass_id) = self.node_map.get(&key, &ctx) {
            return *eclass_id;
        }

        // If that didn't work, we need to create a new eclass.
        let enodes = self.node_arena.single(node);
        let eclass_id = self.classes.push(EClass {
            enodes,
            parents: EntityList::default(),
        });
        self.union.add(eclass_id);

        // Add to interning map with a NodeKey referring to the eclass.
        let key = NodeKey::from_eclass_node(eclass_id, 0);
        let mut ctx = NodeKeyCtx {
            classes: &self.classes,
            arena: &self.node_arena,
            node_ctx,
        };
        self.node_map.insert(key, eclass_id, &mut ctx);

        // For each child ID in the node, append this new enode to the
        // parents list.
        for child_idx in 0..num_args {
            let child_eclass = node_ctx.children(self.node(eclass_id, 0))[child_idx];
            self.classes[child_eclass]
                .parents
                .push(eclass_id, &mut self.parent_pool);
        }

        eclass_id
    }

    fn node<'a>(&'a self, eclass: Id, node_index: usize) -> &'a L::Node {
        &self.classes[eclass].enodes.as_slice(&self.node_arena)[node_index]
    }

    fn node_mut<'a>(&'a mut self, eclass: Id, node_index: usize) -> &'a mut L::Node {
        &mut self.classes[eclass]
            .enodes
            .as_mut_slice(&mut self.node_arena)[node_index]
    }

    pub fn union(&mut self, a: Id, b: Id, ctx: &mut L) {
        let a = self.union.find_and_update(a);
        let b = self.union.find_and_update(b);
        if a == b {
            return;
        }

        // Pick the larger and smaller parent-sets respectively so
        // that we have to update fewer parent-lists.
        let (union_into, union_from) = if self.classes[a].parents.len(&self.parent_pool)
            >= self.classes[b].parents.len(&self.parent_pool)
        {
            (a, b)
        } else {
            (b, a)
        };

        // Do the union-find "union" operation itself. This is what
        // makes the eclasses equivalent.
        self.union.union(union_into, union_from);

        // Take ownership of the enode lists.
        let into_enodes = std::mem::take(&mut self.classes[union_into].enodes);
        let from_enodes = std::mem::take(&mut self.classes[union_from].enodes);

        // Unregister all enodes in `from_enodes` from the dedup
        // hashmap.
        for enode in from_enodes.as_slice(&self.node_arena) {
            let key = NodeKey::from_ref(enode);
            let ctx = NodeKeyCtx {
                classes: &self.classes,
                arena: &self.node_arena,
                node_ctx: ctx,
            };
            self.node_map.remove(&key, &ctx);
        }

        todo!();

        // Likewise, append the parent-list and deduplicate.
        //
        // Note that this `.clone()` is a cheap clone of the reference
        // into the pool.
        let other_parent_list = self.classes[union_from].parents.clone();
        self.classes[union_into]
            .parents
            .append_list(&other_parent_list, &mut self.parent_pool);
        self.classes[union_into].parents.sort(&mut self.parent_pool);
        self.classes[union_into]
            .parents
            .remove_dups(&mut self.parent_pool);

        // Place all parents of `union_from` in the pending-update
        // list.
        self.pending_child_updates.extend(
            other_parent_list
                .as_slice(&self.parent_pool)
                .iter()
                .cloned(),
        );
    }

    /// Rebuild the egraph. Append to a list of eclasses that have
    /// changed, requiring new rewrite rule applications.
    pub fn rebuild(&mut self, _changed: &mut Vec<Id>) {
        todo!()
    }

    /// Get the enodes for a given eclass.
    pub fn enodes<'a>(&'a self, eclass: Id) -> &'a [L::Node] {
        self.classes[eclass].enodes.as_slice(&self.node_arena)
    }
}
