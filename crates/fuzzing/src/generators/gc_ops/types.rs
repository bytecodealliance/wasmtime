//! Types for the `gc` operations.

use crate::generators::gc_ops::limits::GcOpsLimits;
use crate::generators::gc_ops::ops::GcOp;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use wasmtime_environ::graphs::{Dfs, DfsEvent, Graph};

/// Identifies a `(rec ...)` group.
#[derive(
    Debug, Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct RecGroupId(pub(crate) u32);

/// Identifies a type within a rec group.
#[derive(
    Debug, Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct TypeId(pub(crate) u32);

/// A struct type definition.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StructType {}

/// A composite type: currently only structs.
#[derive(Debug, Serialize, Deserialize)]
pub enum CompositeType {
    /// A struct composite type.
    Struct(StructType),
}

/// A sub-type definition (the per-type payload).
#[derive(Debug, Serialize, Deserialize)]
pub struct SubType {
    pub(crate) is_final: bool,
    pub(crate) supertype: Option<TypeId>,
    pub(crate) composite_type: CompositeType,
}

/// Supertype graph: edges go from a type to its supertype.
struct SupertypeGraph<'a> {
    type_defs: &'a BTreeMap<TypeId, SubType>,
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

/// Rec-group dependency graph: group A depends on group B when a type
/// in A has a supertype in B.
struct RecGroupGraph<'a> {
    type_defs: &'a BTreeMap<TypeId, SubType>,
    rec_groups: &'a BTreeMap<RecGroupId, BTreeSet<TypeId>>,
    type_to_group: &'a BTreeMap<TypeId, RecGroupId>,
}

impl Graph<RecGroupId> for RecGroupGraph<'_> {
    type NodesIter<'a>
        = std::iter::Copied<std::collections::btree_map::Keys<'a, RecGroupId, BTreeSet<TypeId>>>
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
                if let Some(super_ty) = self.type_defs.get(&ty).and_then(|d| d.supertype) {
                    if let Some(&super_group) = self.type_to_group.get(&super_ty) {
                        if super_group != group {
                            deps.insert(super_group);
                        }
                    }
                }
            }
        }

        deps.into_iter().collect::<Vec<_>>().into_iter()
    }
}

/// All type and rec-group state.
///
/// Rec groups own sets of [`TypeId`]s; moving a type between groups is
/// just a set remove + set insert with no cascading index fixups.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Types {
    /// Map from rec-group id to the set of types it contains.
    pub(crate) rec_groups: BTreeMap<RecGroupId, BTreeSet<TypeId>>,
    /// Map from type id to its definition.
    pub(crate) type_defs: BTreeMap<TypeId, SubType>,
}

impl Types {
    /// Create empty type state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a fresh [`RecGroupId`] not already in use.
    pub fn fresh_rec_group_id(&self, rng: &mut mutatis::Rng) -> RecGroupId {
        for _ in 0..1000 {
            let id = RecGroupId(rng.gen_u32());
            if !self.rec_groups.contains_key(&id) {
                return id;
            }
        }
        panic!("failed to generate a new RecGroupId in 1000 iterations");
    }

    /// Return a fresh [`TypeId`] not already in use.
    pub fn fresh_type_id(&self, rng: &mut mutatis::Rng) -> TypeId {
        for _ in 0..1000 {
            let id = TypeId(rng.gen_u32());
            if !self.type_defs.contains_key(&id) {
                return id;
            }
        }
        panic!("failed to generate a new TypeId in 1000 iterations");
    }

    /// Insert an empty rec group. Returns `true` if it was newly inserted.
    pub fn insert_rec_group(&mut self, id: RecGroupId) -> bool {
        match self.rec_groups.entry(id) {
            Entry::Vacant(e) => {
                e.insert(BTreeSet::new());
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Insert an empty struct type into the given rec group.
    ///
    /// The rec group must already exist.
    pub fn insert_empty_struct(
        &mut self,
        id: TypeId,
        group: RecGroupId,
        is_final: bool,
        supertype: Option<TypeId>,
    ) {
        self.rec_groups
            .get_mut(&group)
            .expect("rec group must exist")
            .insert(id);
        self.type_defs.insert(
            id,
            SubType {
                is_final,
                supertype,
                composite_type: CompositeType::Struct(StructType::default()),
            },
        );
    }

    /// Remove a type from its rec group and from `type_defs`.
    pub fn remove_type(&mut self, id: TypeId) {
        self.type_defs.remove(&id);
        for members in self.rec_groups.values_mut() {
            members.remove(&id);
        }
    }

    /// Find which rec group a type belongs to, if any.
    pub fn rec_group_of(&self, id: TypeId) -> Option<RecGroupId> {
        self.rec_groups
            .iter()
            .find(|(_, members)| members.contains(&id))
            .map(|(gid, _)| *gid)
    }

    /// Returns true iff `sub` is the same type as, or a subtype of, `sup`.
    ///
    /// Walks the supertype chain from `sub` upward looking for `sup`.
    pub fn is_subtype(&self, mut sub: TypeId, sup: TypeId) -> bool {
        loop {
            if sub == sup {
                return true;
            }

            let next = match self.type_defs.get(&sub).and_then(|d| d.supertype) {
                Some(t) => t,
                None => return false,
            };
            sub = next;
        }
    }

    /// Return the type encoding order grouped by rec group.
    ///
    /// Rec groups are emitted in topological order (dependencies first),
    /// and within each group members are sorted by the global supertype-
    /// first topological order.
    pub(crate) fn encoding_order_grouped(
        &self,
        out: &mut Vec<(RecGroupId, Vec<TypeId>)>,
        type_to_group: &BTreeMap<TypeId, RecGroupId>,
    ) {
        let mut type_order = Vec::with_capacity(self.type_defs.len());
        self.sort_types_topo(&mut type_order);

        let type_position: HashMap<TypeId, usize> = type_order
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();

        let mut group_order = Vec::with_capacity(self.rec_groups.len());
        self.sort_rec_groups_topo(&mut group_order, type_to_group);

        out.clear();
        out.reserve(group_order.len());
        for gid in group_order {
            if let Some(member_set) = self.rec_groups.get(&gid) {
                let mut members: Vec<TypeId> = member_set.iter().copied().collect();
                members.sort_by_key(|tid| type_position[tid]);
                out.push((gid, members));
            }
        }
    }

    /// Topological sort of types by their supertype (supertype before subtype).
    pub fn sort_types_topo(&self, out: &mut Vec<TypeId>) {
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

    /// Topological sort of rec groups: if a type in group G has a
    /// supertype in group H, then H appears before G in the output.
    pub fn sort_rec_groups_topo(
        &self,
        out: &mut Vec<RecGroupId>,
        type_to_group: &BTreeMap<TypeId, RecGroupId>,
    ) {
        let graph = RecGroupGraph {
            type_defs: &self.type_defs,
            rec_groups: &self.rec_groups,
            type_to_group,
        };

        let mut dfs = Dfs::new(graph.nodes());
        let mut seen = BTreeSet::new();

        out.clear();
        out.reserve(self.rec_groups.len());

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

    /// Break cycles in the [type -> supertype] graph by dropping some supertype edges.
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

    /// Build a reverse map from type id to its owning rec group.
    pub(crate) fn type_to_group_map(&self) -> BTreeMap<TypeId, RecGroupId> {
        self.rec_groups
            .iter()
            .flat_map(|(&gid, members)| members.iter().map(move |&tid| (tid, gid)))
            .collect()
    }

    /// Break cycles in the rec-group dependency graph by dropping cross-group
    /// supertype edges that are DFS back edges.
    pub fn break_rec_group_cycles(&mut self, type_to_group: &BTreeMap<TypeId, RecGroupId>) {
        let graph = RecGroupGraph {
            type_defs: &self.type_defs,
            rec_groups: &self.rec_groups,
            type_to_group,
        };

        let mut seen = BTreeSet::new();
        let mut back_edges: BTreeSet<(RecGroupId, RecGroupId)> = BTreeSet::new();
        let mut dfs = Dfs::default();

        for &root in self.rec_groups.keys() {
            if seen.contains(&root) {
                continue;
            }
            dfs.add_root(root);
            let mut active = BTreeSet::new();

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
                            back_edges.insert((from, to));
                        }
                    }
                }
            }
        }

        // Drop supertype edges that correspond to back edges.
        if !back_edges.is_empty() {
            for (&tid, def) in self.type_defs.iter_mut() {
                if let Some(st) = def.supertype {
                    if let (Some(&sg), Some(&spg)) =
                        (type_to_group.get(&tid), type_to_group.get(&st))
                    {
                        if back_edges.contains(&(sg, spg)) {
                            def.supertype = None;
                        }
                    }
                }
            }
        }
    }

    /// Fix up the types to ensure they are within the limits.
    pub fn fixup(
        &mut self,
        limits: &GcOpsLimits,
        encoding_order_grouped: &mut Vec<(RecGroupId, Vec<TypeId>)>,
    ) {
        let max_rec_groups =
            usize::try_from(limits.max_rec_groups).expect("max_rec_groups is too large");
        let max_types = usize::try_from(limits.max_types).expect("max_types is too large");

        // 1. Trim excess types.
        while self.type_defs.len() > max_types {
            if let Some((tid, _)) = self.type_defs.pop_last() {
                for members in self.rec_groups.values_mut() {
                    members.remove(&tid);
                }
            }
        }

        // 2. Drop dangling references and deduplicate across groups.
        let mut seen = BTreeSet::new();
        for members in self.rec_groups.values_mut() {
            members.retain(|tid| self.type_defs.contains_key(tid) && seen.insert(*tid));
        }

        // 3. Trim excess rec groups.
        while self.rec_groups.len() > max_rec_groups {
            self.rec_groups.pop_last();
        }

        // 4. Find all orphans (from trimmed groups or never in any group).
        let housed: BTreeSet<TypeId> = self
            .rec_groups
            .values()
            .flat_map(|m| m.iter().copied())
            .collect();
        let orphans: Vec<TypeId> = self
            .type_defs
            .keys()
            .filter(|tid| !housed.contains(tid))
            .copied()
            .collect();

        // 5. Adopt orphans or drop them.
        if let Some(first_members) = self.rec_groups.values_mut().next() {
            first_members.extend(orphans);
        } else {
            for tid in &orphans {
                self.type_defs.remove(tid);
            }
        }

        // 6. Clear supertypes that reference removed types.
        let valid_type_ids: BTreeSet<TypeId> = self.type_defs.keys().copied().collect();
        for def in self.type_defs.values_mut() {
            if let Some(st) = def.supertype {
                if !valid_type_ids.contains(&st) {
                    def.supertype = None;
                }
            }
        }

        // 7. A subtype cannot have a final supertype.
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

        // 8. Break supertype cycles and rec-group dependency cycles.
        self.break_supertype_cycles();
        let type_to_group = self.type_to_group_map();
        self.break_rec_group_cycles(&type_to_group);

        debug_assert!(self.is_well_formed(limits));

        // 9. Compute encoding order (reuses type_to_group from step 8).
        self.encoding_order_grouped(encoding_order_grouped, &type_to_group);
    }

    /// Check if the types are well-formed and within configured limits, i.e.
    /// rec/type counts are within limits,
    /// every type belongs to exactly one rec group,
    /// and every rec group member must exist in type_defs.
    fn is_well_formed(&self, limits: &GcOpsLimits) -> bool {
        if self.rec_groups.len()
            > usize::try_from(limits.max_rec_groups).expect("max_rec_groups is too large")
        {
            log::debug!("[-] Failed: rec_groups.len() > max_rec_groups");
            return false;
        }
        if self.type_defs.len() > usize::try_from(limits.max_types).expect("max_types is too large")
        {
            log::debug!("[-] Failed: type_defs.len() > max_types");
            return false;
        }
        let mut all = BTreeSet::new();
        for members in self.rec_groups.values() {
            for tid in members {
                if !self.type_defs.contains_key(tid) {
                    log::debug!("[-] Failed: type_defs.contains_key(tid) is false");
                    return false;
                }
                if !all.insert(*tid) {
                    log::debug!("[-] Failed: all.insert(tid) is false");
                    return false;
                }
            }
        }
        if !self.type_defs.keys().all(|tid| all.contains(tid)) {
            log::debug!("[-] Failed: type_defs.keys().all(|tid| all.contains(tid)) is false");
            return false;
        }
        // Every supertype must exist and must not be final.
        for (&tid, def) in &self.type_defs {
            if let Some(st) = def.supertype {
                match self.type_defs.get(&st) {
                    None => {
                        log::debug!("[-] Failed: supertype {st:?} missing for subtype {tid:?}");
                        return false;
                    }
                    Some(super_def) if super_def.is_final => {
                        log::debug!("[-] Failed: subtype {tid:?} has final supertype {st:?}");
                        return false;
                    }
                    _ => {}
                }
            }
        }
        true
    }
}

/// Tracks the required operand type on the abstract value stack.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StackType {
    /// `externref`.
    ExternRef,
    /// `(ref $*)` — optionally with a concrete type index.
    Struct(Option<u32>),
}

impl StackType {
    /// Ensure the top of `stack` satisfies `req`, emitting fixup ops as needed.
    pub fn fixup(
        req: Option<StackType>,
        stack: &mut Vec<StackType>,
        out: &mut Vec<GcOp>,
        num_types: u32,
        types: &Types,
        encoding_order: &[TypeId],
    ) {
        log::trace!(
            "[StackType::fixup] enter req={req:?} num_types={num_types} stack_len={} stack={stack:?}",
            stack.len()
        );
        let mut result_types = Vec::new();
        match req {
            None => {
                if stack.is_empty() {
                    log::trace!("[StackType::fixup] None: empty stack -> emit NullExtern");
                    Self::emit(GcOp::NullExtern, stack, out, num_types, &mut result_types);
                }
                let popped = stack.pop();
                log::trace!("[StackType::fixup] None: pop -> {popped:?} stack={stack:?}");
            }
            Some(Self::ExternRef) => match stack.last() {
                Some(Self::ExternRef) => {
                    log::trace!("[StackType::fixup] ExternRef: top ok -> pop");
                    stack.pop();
                }
                other => {
                    log::trace!(
                        "[StackType::fixup] ExternRef: mismatch top={other:?} -> emit NullExtern+pop"
                    );
                    Self::emit(GcOp::NullExtern, stack, out, num_types, &mut result_types);
                    let popped = stack.pop();
                    log::trace!(
                        "[StackType::fixup] ExternRef: after emit pop -> {popped:?} stack={stack:?}"
                    );
                }
            },
            Some(Self::Struct(wanted)) => {
                let ok = match (wanted, stack.last()) {
                    (Some(wanted), Some(Self::Struct(Some(actual)))) => {
                        let sub = encoding_order
                            .get(usize::try_from(*actual).expect("invalid type index"))
                            .copied();
                        let sup = encoding_order
                            .get(usize::try_from(wanted).expect("invalid type index"))
                            .copied();
                        let st = match (sub, sup) {
                            (Some(sub), Some(sup)) => types.is_subtype(sub, sup),
                            _ => false,
                        };
                        log::trace!(
                            "[StackType::fixup] Struct: actual={actual} wanted={wanted} is_subtype={st}"
                        );
                        st
                    }
                    (None, Some(Self::Struct(_))) => {
                        log::trace!(
                            "[StackType::fixup] Struct: abstract wanted, concrete stack -> ok"
                        );
                        true
                    }
                    _ => {
                        log::trace!(
                            "[StackType::fixup] Struct: no match wanted={wanted:?} last={:?} -> ok=false",
                            stack.last()
                        );
                        false
                    }
                };

                if ok {
                    let popped = stack.pop();
                    log::trace!("[StackType::fixup] Struct: ok -> pop {popped:?} stack={stack:?}");
                } else {
                    match wanted {
                        // When num_types == 0, GcOp::fixup() should have dropped the ops
                        // that require a concrete type.
                        // But it keeps the ops that work with abstract types.
                        // Since our mutator can legally remove all the types,
                        // StackType::fixup() should insert GcOp::NullStruct()
                        // to satisfy the undropped ops that work with abstract types.
                        None => {
                            log::trace!(
                                "[StackType::fixup] Struct synthesize NullStruct stack_before={stack:?}"
                            );
                            Self::emit(GcOp::NullStruct, stack, out, num_types, &mut result_types);
                            let popped = stack.pop();
                            log::trace!(
                                "[StackType::fixup] NullStruct: after emit pop -> {popped:?} stack={stack:?}"
                            );
                        }
                        Some(t) => {
                            debug_assert_ne!(
                                num_types, 0,
                                "typed struct requirement with num_types == 0; op should have been removed"
                            );
                            let t = Self::clamp(t, num_types);
                            log::trace!(
                                "[StackType::fixup] Struct synthesize StructNew type_index={t} stack_before={stack:?}"
                            );
                            Self::emit(
                                GcOp::StructNew { type_index: t },
                                stack,
                                out,
                                num_types,
                                &mut result_types,
                            );
                            log::trace!(
                                "[StackType::fixup] StructNew: after emit stack={stack:?} (next: pop operand)"
                            );
                            let popped = stack.pop();
                            log::trace!(
                                "[StackType::fixup] StructNew: pop -> {popped:?} stack={stack:?}"
                            );
                        }
                    }
                }
            }
        }
        log::trace!(
            "[StackType::fixup] leave stack_len={} stack={stack:?} out_len={}",
            stack.len(),
            out.len()
        );
    }

    /// Emit an opcode and update the stack.
    pub(crate) fn emit(
        op: GcOp,
        stack: &mut Vec<Self>,
        out: &mut Vec<GcOp>,
        num_types: u32,
        result_types: &mut Vec<Self>,
    ) {
        log::trace!(
            "[StackType::emit] op={op:?} stack_len_before={} num_types={num_types}",
            stack.len()
        );
        out.push(op);
        result_types.clear();
        op.result_types(result_types);
        for ty in result_types {
            let clamped_ty = match ty {
                Self::Struct(Some(t)) => Self::Struct(Some(Self::clamp(*t, num_types))),
                other => *other,
            };
            log::trace!("[StackType::emit] push result {clamped_ty:?}");
            stack.push(clamped_ty);
        }
        log::trace!("[StackType::emit] leave stack={stack:?}");
    }

    /// Fixup for cast ops: ensures the sub/super type relationship actually
    /// holds. Always repairs the op rather than dropping it.
    ///
    /// For upcast the operand (sub) is on the stack, so we keep sub fixed
    /// and adjust super. For downcast the operand (super) is on the stack,
    /// so we keep super fixed and adjust sub.
    pub fn fixup_cast(op: GcOp, types: &Types, encoding_order: &[TypeId]) -> GcOp {
        match op {
            GcOp::RefCastUpward {
                sub_type_index,
                super_type_index,
            } => {
                // Operand is sub (on the stack) — keep it, fix super.
                let super_type_index = Self::find_supertype_of(
                    sub_type_index,
                    super_type_index,
                    types,
                    encoding_order,
                );
                GcOp::RefCastUpward {
                    sub_type_index,
                    super_type_index,
                }
            }
            GcOp::RefCastDownward {
                sub_type_index,
                super_type_index,
            } => {
                // Operand is super (on the stack) — keep it, fix sub.
                let sub_type_index =
                    Self::find_subtype_of(super_type_index, sub_type_index, types, encoding_order);
                GcOp::RefCastDownward {
                    sub_type_index,
                    super_type_index,
                }
            }
            other => other,
        }
    }

    /// Given a sub type on the stack, find a valid super_type_index such
    /// that sub <: super. Keeps sub fixed. Falls back to self-cast.
    fn find_supertype_of(
        sub_type_index: u32,
        super_type_index: u32,
        types: &Types,
        encoding_order: &[TypeId],
    ) -> u32 {
        if let (Some(&sub_tid), Some(&super_tid)) = (
            encoding_order
                .get(usize::try_from(sub_type_index).expect("sub_type_index is out of bounds")),
            encoding_order
                .get(usize::try_from(super_type_index).expect("super_type_index is out of bounds")),
        ) {
            // Already valid.
            if types.is_subtype(sub_tid, super_tid) {
                return super_type_index;
            }
            // Try sub's direct supertype.
            if let Some(actual_super) = types.type_defs.get(&sub_tid).and_then(|d| d.supertype) {
                if let Some(idx) = encoding_order.iter().position(|&t| t == actual_super) {
                    return u32::try_from(idx).expect("too many types for index");
                }
            }
        }
        // Self-cast.
        sub_type_index
    }

    /// Given a super type on the stack, find a valid sub_type_index such
    /// that sub <: super. Keeps super fixed. Falls back to self-cast.
    fn find_subtype_of(
        super_type_index: u32,
        sub_type_index: u32,
        types: &Types,
        encoding_order: &[TypeId],
    ) -> u32 {
        if let (Some(&sub_tid), Some(&super_tid)) = (
            encoding_order
                .get(usize::try_from(sub_type_index).expect("sub_type_index is out of bounds")),
            encoding_order
                .get(usize::try_from(super_type_index).expect("super_type_index is out of bounds")),
        ) {
            // Already valid.
            if types.is_subtype(sub_tid, super_tid) {
                return sub_type_index;
            }
            // Try to find any direct subtype of super.
            for (idx, tid) in encoding_order.iter().enumerate() {
                if let Some(def) = types.type_defs.get(tid) {
                    if def.supertype == Some(super_tid) {
                        return u32::try_from(idx).expect("too many types for index");
                    }
                }
            }
        }
        // Self-cast.
        super_type_index
    }

    /// Clamp a type index to the number of types.
    fn clamp(t: u32, n: u32) -> u32 {
        if n == 0 { 0 } else { t % n }
    }
}
