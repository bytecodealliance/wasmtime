//! Egraph implementation.

use crate::batched_workset::BatchedWorkset;
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
    /// Set of eclass IDs that have been merged into other eclasses,
    /// for deferred merge processing.
    pending_merges: BatchedWorkset<Id>,
    /// Set of node IDs whose children have been updated. These need
    /// to be re-canonicalized in `.rebuild()`.
    pending_recanonicalizations: BatchedWorkset<Id>,
    /// List of eclass IDs that have been updated (have a new node)
    /// since last rebuild. These need to be processed by any rewrite
    /// rules.
    pending_dirty_classes: BatchedWorkset<Id>,
}

/// A reference to a node.
struct NodeKey<'a, L: Language> {
    bits: u64,
    _phantom: PhantomData<&'a L::Node>,
}

impl<'a, L: Language> PartialEq for NodeKey<'a, L> {
    fn eq(&self, other: &Self) -> bool {
        self.bits == other.bits
    }
}
impl<'a, L: Language> Eq for NodeKey<'a, L> {}

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
            pending_merges: BatchedWorkset::default(),
            pending_recanonicalizations: BatchedWorkset::default(),
            pending_dirty_classes: BatchedWorkset::default(),
        }
    }

    pub fn with_capacity(nodes: usize) -> Self {
        Self {
            node_arena: BumpArena::arena_with_capacity(nodes),
            node_map: CtxHashMap::with_capacity(nodes),
            union: UnionFind::with_capacity(nodes),
            classes: PrimaryMap::with_capacity(nodes),
            parent_pool: ListPool::new(),
            pending_merges: BatchedWorkset::default(),
            pending_recanonicalizations: BatchedWorkset::default(),
            pending_dirty_classes: BatchedWorkset::default(),
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

        self.pending_dirty_classes.add(eclass_id);

        eclass_id
    }

    fn node<'a>(&'a self, eclass: Id, node_index: usize) -> &'a L::Node {
        &self.classes[eclass].enodes.as_slice(&self.node_arena)[node_index]
    }

    /// Do a merge with deferred fixups: merge the eclasses in the
    /// union-find data structure, and enqueue the merge action itself.
    pub fn union(&mut self, a: Id, b: Id, _ctx: &mut L) {
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
        self.pending_merges.add(union_from);
    }

    /// Process a merge, actually combining the enode and parent lists.
    fn do_merge(&mut self, union_into: Id, union_from: Id, ctx: &mut L) {
        // Take ownership of the enode lists.
        let into_enodes = std::mem::take(&mut self.classes[union_into].enodes);
        let from_enodes = std::mem::take(&mut self.classes[union_from].enodes);

        // For each node in `from_enodes`, rewrite the NodeKey to the
        // into-class and new enode index.
        let from_enodes_slice = from_enodes.as_slice(&self.node_arena);
        for i in 0..from_enodes.len() {
            let node_key = NodeKey::from_eclass_node(union_from, i);
            let new_node_key = NodeKey::from_eclass_node(union_into, into_enodes.len() + i);
            let hash_key = NodeKey::from_ref(&from_enodes_slice[i]);
            let ctx = NodeKeyCtx {
                classes: &self.classes,
                arena: &self.node_arena,
                node_ctx: ctx,
            };
            self.node_map
                .rewrite_raw_key(&hash_key, &node_key, new_node_key, &ctx);
        }

        // Append the enode list and place it into `union_into`.
        let enodes = self.node_arena.append(into_enodes, from_enodes);
        self.classes[union_into].enodes = enodes;

        // Take the parent lists and append them.
        let mut into_parents = std::mem::take(&mut self.classes[union_into].parents);
        let from_parents = std::mem::take(&mut self.classes[union_from].parents);

        into_parents.append_list(&from_parents, &mut self.parent_pool);
        self.classes[union_into].parents = into_parents;

        // Place all `from_parents` in the pending-child-update list.
        for &parent in from_parents.as_slice(&self.parent_pool) {
            self.pending_recanonicalizations.add(parent);
        }

        // Place this node in the pending-dirty-classes list.
        self.pending_dirty_classes.add(union_into);
    }

    /// Rebuild the egraph.
    pub fn rebuild(&mut self, ctx: &mut L) {
        // While there are nodes with pending merges, perform them;
        // while there are pending recanonicalizations, perform
        // them. Merges may create recanonicalizations for parents,
        // and recanonicalizations may result in additional merges, so
        // we run both together in a fixpoint loop.
        while !self.pending_merges.is_empty() || !self.pending_recanonicalizations.is_empty() {
            let mut batch = self.pending_merges.take_batch();
            for union_from in batch.batch() {
                let union_into = self.union.find_and_update(union_from);
                if union_into != union_from {
                    self.do_merge(union_into, union_from, ctx);
                }
            }
            self.pending_merges.reuse(batch);

            let mut batch = self.pending_recanonicalizations.take_batch();
            for node in batch.batch() {
                self.do_recanonicalize(node, ctx);
            }
            self.pending_recanonicalizations.reuse(batch);
        }
    }

    fn do_recanonicalize(&mut self, eclass: Id, ctx: &mut L) {
        // For each node in the eclass: (i) remove from dedup, (ii)
        // canonicalize all arg Ids according to the union-find, (iii)
        // re-intern. If Re-interning hits another node already in
        // dedup, then we've stumbled upon a recursive merging (merged
        // children result in merged parents); enqueue that as a
        // pending merge.
        let n_nodes = self.classes[eclass].enodes.len();
        let mut out_idx = 0;
        for node_idx in 0..n_nodes {
            // Remove from dedup hashmap.
            let key = NodeKey::from_eclass_node(eclass, node_idx);
            let keyctx = NodeKeyCtx {
                classes: &self.classes,
                arena: &self.node_arena,
                node_ctx: ctx,
            };
            self.node_map.remove(&key, &keyctx);

            // Recanonicalize all arg eclass IDs.
            let nodes = self.classes[eclass]
                .enodes
                .as_mut_slice(&mut self.node_arena);
            let mut changed = false;
            for arg in ctx.children_mut(&mut nodes[node_idx]) {
                let orig_arg = *arg;
                *arg = self.union.find_and_update(*arg);
                if *arg != orig_arg {
                    changed = true;
                }
            }

            // Re-insert into dedup hashmap. If the newly-edited node
            // is now a duplicate, we've found an eclass merge. Do the
            // union-find, enqueue the actual node-list/parent-list
            // merge, and skip bumping `out_idx` (i.e., remove this
            // duplicate from our list). Otherwise, we "emit" it by
            // bumping `out_idx` and, if this has fallen behind
            // `node_idx`, shifting the node backward.
            let key = NodeKey::from_eclass_node(eclass, node_idx);
            let keyctx = NodeKeyCtx {
                classes: &self.classes,
                arena: &self.node_arena,
                node_ctx: ctx,
            };
            if let Some(&dup_eclass) = self.node_map.get(&key, &keyctx) {
                self.union.union(dup_eclass, eclass);
                self.pending_merges.add(eclass);
                self.pending_dirty_classes.add(dup_eclass);
            } else {
                let idx = out_idx;
                if out_idx < node_idx {
                    self.classes[eclass]
                        .enodes
                        .as_mut_slice(&mut self.node_arena)
                        .swap(out_idx, node_idx);
                }
                out_idx += 1;
                let key = NodeKey::from_eclass_node(eclass, idx);
                let mut keyctx = NodeKeyCtx {
                    classes: &self.classes,
                    arena: &self.node_arena,
                    node_ctx: ctx,
                };
                self.node_map.insert(key, eclass, &mut keyctx);

                if changed {
                    self.pending_dirty_classes.add(eclass);
                }
            }
        }
    }

    /// Provide a list of all eclass IDs that are "dirty" (have
    /// changed or were newly added).
    pub fn dirty_classes_workset(&mut self) -> &mut BatchedWorkset<Id> {
        &mut self.pending_dirty_classes
    }

    /// Get the enodes for a given eclass.
    pub fn enodes<'a>(&'a self, eclass: Id) -> &'a [L::Node] {
        self.classes[eclass].enodes.as_slice(&self.node_arena)
    }
}
