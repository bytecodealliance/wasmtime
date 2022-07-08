//! Egraph implementation.

use crate::unionfind::UnionFind;
use crate::{Id, Language, NodeId};
use cranelift_entity::{EntityList, EntityRef, ListPool, PrimaryMap};
use indexmap::IndexMap;

/// An egraph.
#[derive(Clone, Debug)]
pub struct EGraph<L: Language> {
    /// The single store for node contents (as keys), and a mapping
    /// from each node to the eclass in which it appears. Can also be
    /// indexed by NodeID.
    nodes: IndexMap<L, Id>,
    /// Union-find data structure representing eclass merging.
    union: UnionFind,
    /// Eclass definitions. Each eclass consists of a list of nodes,
    /// and of parent nodes that refer to this eclass.
    classes: PrimaryMap<Id, EClass>,
    /// The pool in which we store lists of enode IDs, used for eclass
    /// contents.
    enode_list_pool: ListPool<NodeId>,
    /// List of node IDs whose children have been updated. These need
    /// to be re-canonicalized in `.rebuild()`.
    pending_child_updates: Vec<NodeId>,
    /// List of eclass IDs that have been updated (have a new node)
    /// since last rebuild. These need to be processed by any rewrite
    /// rules.
    #[allow(dead_code)]
    pending_dirty_classes: Vec<Id>,
}

#[derive(Clone, Debug)]
pub struct EClass {
    enodes: EntityList<NodeId>,
    parents: EntityList<NodeId>,
}

impl<L: Language> EGraph<L> {
    pub fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
            union: UnionFind::new(),
            classes: PrimaryMap::new(),
            enode_list_pool: ListPool::new(),
            pending_child_updates: vec![],
            pending_dirty_classes: vec![],
        }
    }

    /// Add a new node.
    pub fn add(&mut self, mut node: L) -> Id {
        // Canonicalize all argument eclass IDs. This is relatively
        // cheap if the user has already done so, so we do this
        // unconditionally.
        for child in node.children_mut() {
            let id = self.union.find_and_update(*child);
            *child = id;
        }

        // First, intern the node. If it already exists, return the
        // eclass ID for it. This takes ownership of the sole copy of
        // the node data; we never clone it.
        let (enode_id, eclass_id) = match self.nodes.entry(node) {
            indexmap::map::Entry::Occupied(o) => {
                return *o.get();
            }
            indexmap::map::Entry::Vacant(v) => {
                let class_id = self.classes.push(EClass {
                    enodes: EntityList::default(),
                    parents: EntityList::default(),
                });
                self.union.add(class_id);
                let node_id = NodeId(v.index() as u32);
                (node_id, *v.insert(class_id))
            }
        };

        // Add the node to the new eclass.
        self.classes[eclass_id].enodes =
            EntityList::from_slice(&[enode_id], &mut self.enode_list_pool);
        // For each child ID in the node, append this new enode to the
        // parents list.
        let node = self.nodes.get_index(enode_id.index()).unwrap().0;
        for &child_class in node.children() {
            self.classes[child_class]
                .parents
                .push(enode_id, &mut self.enode_list_pool);
        }

        eclass_id
    }

    pub fn union(&mut self, a: Id, b: Id) {
        let a = self.union.find_and_update(a);
        let b = self.union.find_and_update(b);
        if a == b {
            return;
        }

        // Pick the larger and smaller parent-sets respectively so
        // that we have to update fewer parent-lists.
        let (union_into, union_from) = if self.classes[a].parents.len(&self.enode_list_pool)
            >= self.classes[b].parents.len(&self.enode_list_pool)
        {
            (a, b)
        } else {
            (b, a)
        };

        // Do the union-find "union" operation itself. This is what
        // makes the eclasses equivalent.
        self.union.union(union_into, union_from);

        // Append the node-list from `union_from` to the node-list in
        // `union_into`. Sort the resulting list and remove
        // duplicates.
        //
        // Note that this `.clone()` clones the entity-list, which is
        // just a small reference into the pool.
        let other_enode_list = self.classes[union_from].enodes.clone();
        self.classes[union_into]
            .enodes
            .append_list(&other_enode_list, &mut self.enode_list_pool);
        self.classes[union_from].enodes = EntityList::default();

        self.classes[union_into]
            .enodes
            .sort(&mut self.enode_list_pool);
        self.classes[union_into]
            .enodes
            .remove_dups(&mut self.enode_list_pool);

        // Likewise, append the parent-list and deduplicate.
        //
        // Note that this `.clone()` is a cheap clone of the reference
        // into the pool.
        let other_parent_list = self.classes[union_from].parents.clone();
        self.classes[union_into]
            .parents
            .append_list(&other_parent_list, &mut self.enode_list_pool);
        self.classes[union_into]
            .parents
            .sort(&mut self.enode_list_pool);
        self.classes[union_into]
            .parents
            .remove_dups(&mut self.enode_list_pool);

        // Place all parents of `union_from` in the pending-update
        // list.
        self.pending_child_updates.extend(
            other_parent_list
                .as_slice(&self.enode_list_pool)
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
    pub fn enodes(&self, eclass: Id) -> impl Iterator<Item = (NodeId, &L)> {
        self.classes[eclass]
            .enodes
            .as_slice(&self.enode_list_pool)
            .iter()
            .map(|&node_id| {
                let node = self.nodes.get_index(node_id.index()).unwrap().0;
                (node_id, node)
            })
    }

    /// Get an individual enode by ID.
    pub fn enode(&self, enode: NodeId) -> &L {
        self.nodes.get_index(enode.index()).unwrap().0
    }
}
