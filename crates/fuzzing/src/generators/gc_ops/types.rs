//! Types for the `gc` operations.

use crate::generators::gc_ops::limits::GcOpsLimits;
use crate::generators::gc_ops::ops::GcOp;
use cranelift_entity::{PrimaryMap, SecondaryMap};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use wasmtime_environ::graphs::{Dfs, DfsEvent, Graph, StronglyConnectedComponents};

/// RecGroup ID struct definition.
#[derive(
    Debug, Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct RecGroupId(pub(crate) u32);

/// TypeID struct definition.
#[derive(
    Debug, Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct TypeId(pub(crate) u32);

/// StructType definition.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StructType {
    // Empty for now; fields will come in a future PR.
}

/// CompsiteType definition.
#[derive(Debug, Serialize, Deserialize)]
pub enum CompositeType {
    /// Struct Type definition.
    Struct(StructType),
}

/// SubType definition
#[derive(Debug, Serialize, Deserialize)]
pub struct SubType {
    pub(crate) rec_group: RecGroupId,
    pub(crate) is_final: bool,
    pub(crate) supertype: Option<TypeId>,
    pub(crate) composite_type: CompositeType,
}
/// Struct types definition.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Types {
    pub(crate) rec_groups: BTreeSet<RecGroupId>,
    pub(crate) type_defs: BTreeMap<TypeId, SubType>,
}

/// Supertype graph definition.
struct SupertypeGraph<'a> {
    type_defs: &'a BTreeMap<TypeId, SubType>,
}

/// Rec-group graph definition.
struct RecGroupGraph<'a> {
    type_defs: &'a BTreeMap<TypeId, SubType>,
    rec_groups: &'a BTreeMap<RecGroupId, Vec<TypeId>>,
}

impl Graph<RecGroupId> for RecGroupGraph<'_> {
    type NodesIter<'a>
        = std::iter::Copied<std::collections::btree_map::Keys<'a, RecGroupId, Vec<TypeId>>>
    where
        Self: 'a;

    fn nodes(&self) -> Self::NodesIter<'_> {
        self.rec_groups.keys().copied()
    }

    type SuccessorsIter<'a>
        = std::vec::IntoIter<RecGroupId>
    where
        Self: 'a;

    fn successors(&self, group: RecGroupId) -> Self::SuccessorsIter<'_> {
        let mut deps = BTreeSet::new();

        if let Some(type_ids) = self.rec_groups.get(&group) {
            for &ty in type_ids {
                if let Some(super_ty) = self.type_defs[&ty].supertype {
                    let super_group = self.type_defs[&super_ty].rec_group;
                    if super_group != group {
                        deps.insert(super_group);
                    }
                }
            }
        }

        deps.into_iter().collect::<Vec<_>>().into_iter()
    }
}

impl Graph<TypeId> for SupertypeGraph<'_> {
    type NodesIter<'a>
        = std::iter::Copied<std::collections::btree_map::Keys<'a, TypeId, SubType>>
    where
        Self: 'a;

    fn nodes(&self) -> Self::NodesIter<'_> {
        self.type_defs.keys().copied()
    }

    type SuccessorsIter<'a>
        = std::option::IntoIter<TypeId>
    where
        Self: 'a;

    fn successors(&self, node: TypeId) -> Self::SuccessorsIter<'_> {
        self.type_defs
            .get(&node)
            .and_then(|def| def.supertype)
            .into_iter()
    }
}

/// Dense rec-group ID struct definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DenseGroupId(u32);
wasmtime_environ::entity_impl!(DenseGroupId);

/// Dense rec-group graph definition.
#[derive(Debug, Default)]
struct DenseRecGroupGraph {
    edges: SecondaryMap<DenseGroupId, Vec<DenseGroupId>>,
}

impl Graph<DenseGroupId> for DenseRecGroupGraph {
    type NodesIter<'a>
        = wasmtime_environ::Keys<DenseGroupId>
    where
        Self: 'a;

    fn nodes(&self) -> Self::NodesIter<'_> {
        self.edges.keys()
    }

    type SuccessorsIter<'a>
        = core::iter::Copied<core::slice::Iter<'a, DenseGroupId>>
    where
        Self: 'a;

    fn successors(&self, node: DenseGroupId) -> Self::SuccessorsIter<'_> {
        self.edges[node].iter().copied()
    }
}

impl Types {
    /// Create a fresh `Types` allocator with no recursive groups defined yet.
    pub fn new() -> Self {
        Self {
            rec_groups: Default::default(),
            type_defs: Default::default(),
        }
    }

    /// Break cycles in the type -> supertype graph by dropping some supertype edges.
    pub fn break_supertype_cycles(&mut self) {
        let graph = SupertypeGraph {
            type_defs: &self.type_defs,
        };

        let mut dfs = Dfs::new(graph.nodes());
        let mut seen = BTreeSet::new();
        let mut active = BTreeSet::new();
        let mut to_clear = BTreeSet::new();

        while let Some(event) = dfs.next(&graph, |id| seen.contains(&id)) {
            match event {
                DfsEvent::Pre(id) => {
                    seen.insert(id);
                    active.insert(id);
                }
                DfsEvent::Post(id) => {
                    active.remove(&id);
                }
                DfsEvent::AfterEdge(from, to) => {
                    if active.contains(&to) {
                        to_clear.insert(from);
                    }
                }
            }
        }

        for id in to_clear {
            if let Some(def) = self.type_defs.get_mut(&id) {
                def.supertype = None;
            }
        }
    }

    /// Topological sort of rec-groups in place.
    pub fn sort_rec_groups_topo(
        &self,
        groups: &mut Vec<RecGroupId>,
        rec_groups: &BTreeMap<RecGroupId, Vec<TypeId>>,
    ) {
        let graph = RecGroupGraph {
            type_defs: &self.type_defs,
            rec_groups,
        };

        let mut dfs = Dfs::new(graph.nodes());
        let mut seen = BTreeSet::new();
        let mut active = BTreeSet::new();

        groups.clear();
        groups.reserve(rec_groups.len());

        while let Some(event) = dfs.next(&graph, |id| seen.contains(&id)) {
            match event {
                DfsEvent::Pre(id) => {
                    seen.insert(id);
                    active.insert(id);
                }
                DfsEvent::Post(id) => {
                    active.remove(&id);
                    groups.push(id);
                }
                DfsEvent::AfterEdge(from, to) => {
                    debug_assert!(
                        !active.contains(&to),
                        "cycle in rec-group dependency graph: {:?} -> {:?}",
                        from,
                        to
                    );
                }
            }
        }
    }

    /// Topological sort of types by their supertype (supertype before subtype) in place.
    pub fn sort_types_by_supertype(&self, out: &mut Vec<TypeId>) {
        let graph = SupertypeGraph {
            type_defs: &self.type_defs,
        };

        let mut dfs = Dfs::new(graph.nodes());
        let mut seen = BTreeSet::new();

        out.clear();
        out.reserve(self.type_defs.len());

        while let Some(event) = dfs.next(&graph, |id| seen.contains(&id)) {
            match event {
                DfsEvent::Pre(id) => {
                    seen.insert(id);
                }
                DfsEvent::Post(id) => {
                    out.push(id);
                }
                DfsEvent::AfterEdge(_, _) => {}
            }
        }
    }

    /// Merge rec-groups that participate in dependency cycles.
    pub fn merge_rec_group_sccs(&mut self) {
        let mut rec_groups: BTreeMap<RecGroupId, Vec<TypeId>> = self
            .rec_groups
            .iter()
            .copied()
            .map(|g| (g, Vec::new()))
            .collect();

        for (&id, def) in &self.type_defs {
            rec_groups.entry(def.rec_group).or_default().push(id);
        }

        let sccs = self.rec_group_sccs(&rec_groups);

        for groups in sccs {
            if groups.len() <= 1 {
                continue;
            }

            let keep = *groups.iter().min().unwrap();

            for &group in &groups {
                if group == keep {
                    continue;
                }

                if let Some(type_ids) = rec_groups.get(&group) {
                    for &ty in type_ids {
                        if let Some(def) = self.type_defs.get_mut(&ty) {
                            def.rec_group = keep;
                        }
                    }
                }

                self.rec_groups.remove(&group);
            }
        }
    }

    /// Find strongly-connected components in the rec-group dependency graph.
    fn rec_group_sccs(
        &self,
        rec_groups: &BTreeMap<RecGroupId, Vec<TypeId>>,
    ) -> Vec<Vec<RecGroupId>> {
        let mut dense_to_group = PrimaryMap::<DenseGroupId, RecGroupId>::new();
        let mut group_to_dense = BTreeMap::<RecGroupId, DenseGroupId>::new();

        for &group in rec_groups.keys() {
            let dense = dense_to_group.push(group);
            group_to_dense.insert(group, dense);
        }

        let mut graph = DenseRecGroupGraph::default();

        for dense in dense_to_group.keys() {
            let _ = &graph.edges[dense];
        }

        for (&group, type_ids) in rec_groups {
            let from = group_to_dense[&group];
            let mut succs = BTreeSet::new();

            for &ty in type_ids {
                if let Some(super_ty) = self.type_defs[&ty].supertype {
                    let super_group = self.type_defs[&super_ty].rec_group;
                    if super_group != group {
                        succs.insert(group_to_dense[&super_group]);
                    }
                }
            }

            graph.edges[from].extend(succs.into_iter());
        }

        let sccs = StronglyConnectedComponents::new(&graph);

        sccs.iter()
            .map(|(_, nodes)| {
                nodes
                    .iter()
                    .copied()
                    .map(|dense| dense_to_group[dense])
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Returns a fresh rec-group id that is not already in use.
    pub fn fresh_rec_group_id(&self, rng: &mut mutatis::Rng) -> RecGroupId {
        for _ in 0..1000 {
            let id = RecGroupId(rng.gen_u32());
            if !self.rec_groups.contains(&id) {
                return id;
            }
        }
        panic!("failed to generate a new RecGroupId in 1000 iterations; bad RNG?");
    }

    /// Returns a fresh type id that is not already in use.
    pub fn fresh_type_id(&self, rng: &mut mutatis::Rng) -> TypeId {
        for _ in 0..1000 {
            let id = TypeId(rng.gen_u32());
            if !self.type_defs.contains_key(&id) {
                return id;
            }
        }
        panic!("failed to generate a new TypeId in 1000 iterations; bad RNG?");
    }

    /// Insert a rec-group id. Returns true if newly inserted, false if it already existed.
    pub fn insert_rec_group(&mut self, id: RecGroupId) -> bool {
        self.rec_groups.insert(id)
    }

    /// Insert an empty struct type with the given rec group, "is_final", and optional supertype.
    pub fn insert_empty_struct(
        &mut self,
        id: TypeId,
        group: RecGroupId,
        is_final: bool,
        supertype: Option<TypeId>,
    ) {
        self.type_defs.insert(
            id,
            SubType {
                rec_group: group,
                is_final,
                supertype,
                composite_type: CompositeType::Struct(StructType::default()),
            },
        );
    }

    /// Fixup type-related inconsistencies.
    pub fn fixup(&mut self, limits: &GcOpsLimits) {
        while self.rec_groups.len() > usize::try_from(limits.max_rec_groups).unwrap() {
            self.rec_groups.pop_last();
        }

        // Drop any types whose rec-group has been trimmed out.
        self.type_defs
            .retain(|_, ty| self.rec_groups.contains(&ty.rec_group));

        // Then enforce the max types limit.
        while self.type_defs.len() > usize::try_from(limits.max_types).unwrap() {
            self.type_defs.pop_last();
        }

        // If supertype is gone, make the current type's supertype None.
        let valid_type_ids: BTreeSet<TypeId> = self.type_defs.keys().copied().collect();
        for def in self.type_defs.values_mut() {
            if let Some(st) = def.supertype {
                if !valid_type_ids.contains(&st) {
                    def.supertype = None;
                }
            }
        }

        // A subtype cannot have a final supertype. Clear supertype when super is final.
        let final_type_ids: BTreeSet<TypeId> = self
            .type_defs
            .iter()
            .filter(|(_, d)| d.is_final)
            .map(|(id, _)| *id)
            .collect();
        for def in self.type_defs.values_mut() {
            if let Some(st) = def.supertype {
                if final_type_ids.contains(&st) {
                    def.supertype = None;
                }
            }
        }

        self.break_supertype_cycles();
        self.merge_rec_group_sccs();

        debug_assert!(
            self.type_defs
                .values()
                .all(|ty| self.rec_groups.contains(&ty.rec_group)),
            "type_defs must only reference existing rec_groups"
        );
    }
}

/// This is used to track the requirements for the operands of an operation.
#[derive(Copy, Clone, Debug)]
pub enum StackType {
    /// `externref`.
    ExternRef,
    /// `(ref $*)`.
    Struct(Option<u32>),
}

impl StackType {
    /// Fixes the stack type to match the given requirement.
    pub fn fixup(
        req: Option<StackType>,
        stack: &mut Vec<StackType>,
        out: &mut Vec<GcOp>,
        num_types: u32,
    ) {
        let mut result_types = Vec::new();
        match req {
            None => {
                if stack.is_empty() {
                    Self::emit(GcOp::NullExtern, stack, out, num_types, &mut result_types);
                }
                stack.pop(); // always consume exactly one value
            }
            Some(Self::ExternRef) => match stack.last() {
                Some(Self::ExternRef) => {
                    stack.pop();
                }
                _ => {
                    Self::emit(GcOp::NullExtern, stack, out, num_types, &mut result_types);
                    stack.pop(); // consume just-synthesized externref
                }
            },
            Some(Self::Struct(wanted)) => {
                let ok = match (wanted, stack.last()) {
                    (Some(wanted), Some(Self::Struct(Some(s)))) => *s == wanted,
                    (None, Some(Self::Struct(_))) => true,
                    _ => false,
                };

                if ok {
                    stack.pop();
                } else {
                    match wanted {
                        // When num_types == 0, GcOp::fixup() should have dropped the ops
                        // that require a concrete type.
                        // But it keeps the ops that work with abstract types.
                        // Since our mutator can legally remove all the types,
                        // StackType::fixup() should insert GcOp::NullStruct()
                        // to satisfy the undropped ops that work with abstract types.
                        None => {
                            Self::emit(GcOp::NullStruct, stack, out, num_types, &mut result_types);
                            stack.pop();
                        }

                        // Typed struct requirement: only satisfiable if we have concrete types.
                        Some(t) => {
                            debug_assert_ne!(
                                num_types, 0,
                                "typed struct requirement with num_types == 0; op should have been removed"
                            );
                            let t = Self::clamp(t, num_types);
                            Self::emit(
                                GcOp::StructNew { type_index: t },
                                stack,
                                out,
                                num_types,
                                &mut result_types,
                            );
                            stack.pop();
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn emit(
        op: GcOp,
        stack: &mut Vec<Self>,
        out: &mut Vec<GcOp>,
        num_types: u32,
        result_types: &mut Vec<Self>,
    ) {
        out.push(op);
        result_types.clear();
        op.result_types(result_types);
        for ty in result_types {
            let clamped_ty = match ty {
                Self::Struct(Some(t)) => Self::Struct(Some(Self::clamp(*t, num_types))),
                other => *other,
            };
            stack.push(clamped_ty);
        }
    }

    fn clamp(t: u32, n: u32) -> u32 {
        if n == 0 { 0 } else { t % n }
    }
}
