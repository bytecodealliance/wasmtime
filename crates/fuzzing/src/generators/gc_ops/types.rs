//! Types for the `gc` operations.

use crate::generators::gc_ops::limits::GcOpsLimits;
use crate::generators::gc_ops::ops::GcOp;
use crate::generators::gc_ops::scc::StronglyConnectedComponents;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

impl Types {
    /// Create a fresh `Types` allocator with no recursive groups defined yet.
    pub fn new() -> Self {
        Self {
            rec_groups: Default::default(),
            type_defs: Default::default(),
        }
    }

    /// Break cycles in supertype edges within each rec-group by dropping some edges.
    pub fn break_type_cycles_in_rec_groups(&mut self) {
        // Kill self-edges to avoid cycles.
        for (id, def) in self.type_defs.iter_mut() {
            if def.supertype == Some(*id) {
                def.supertype = None;
            }
        }

        // Build group -> member list from current truth.
        let mut members: BTreeMap<RecGroupId, Vec<TypeId>> = BTreeMap::new();
        for (id, def) in self.type_defs.iter() {
            members.entry(def.rec_group).or_default().push(*id);
        }

        // For each group, break cycles in the TypeId supertype graph.
        for (_g, ids) in members.iter() {
            if ids.len() <= 1 {
                continue;
            }

            let id_set: BTreeSet<TypeId> = ids.iter().copied().collect();

            // DFS from each node, if we revisit a node in the
            // current path, we found a cycle. Break it by clearing supertype.
            let mut visited = BTreeSet::new();
            for &start in ids {
                if visited.contains(&start) {
                    continue;
                }

                let mut path = Vec::new();
                let mut path_set = BTreeSet::new();
                let mut cur = start;

                loop {
                    if path_set.contains(&cur) {
                        // Found a cycle. Clear supertype to break it.
                        if let Some(def) = self.type_defs.get_mut(&cur) {
                            def.supertype = None;
                        }
                        break;
                    }

                    if visited.contains(&cur) {
                        break;
                    }

                    path.push(cur);
                    path_set.insert(cur);
                    visited.insert(cur);

                    let next = self.type_defs.get(&cur).and_then(|d| d.supertype);
                    match next {
                        Some(st) if id_set.contains(&st) => cur = st,
                        _ => break,
                    }
                }
            }
        }
    }

    /// Get the successors of the given rec-group.
    /// It is used to find the SCCs.
    fn rec_group_successors<'a>(
        &'a self,
        rec_groups: &'a BTreeMap<RecGroupId, Vec<TypeId>>,
        g: RecGroupId,
    ) -> impl Iterator<Item = RecGroupId> + 'a {
        let mut deps = BTreeSet::<RecGroupId>::new();

        for &ty in &rec_groups[&g] {
            if let Some(st) = self.type_defs[&ty].supertype {
                let h = self.type_defs[&st].rec_group;
                if h != g {
                    deps.insert(h);
                }
            }
        }

        deps.into_iter()
    }

    /// Merge rec-groups that participate in dependency cycles.
    pub fn merge_rec_groups_via_scc(&mut self, rec_groups: &BTreeMap<RecGroupId, Vec<TypeId>>) {
        let nodes = rec_groups.keys().copied();
        let sccs =
            StronglyConnectedComponents::new(nodes, |g| self.rec_group_successors(rec_groups, g));

        for groups in sccs.iter() {
            if groups.len() <= 1 {
                continue;
            }

            // Deterministic canonical "keep" group.
            // Smallest RecGroupId in the SCC.
            let keep = *groups.iter().min().unwrap();

            // Merge every other group into "keep" group by rewriting only the members of that group.
            for &g in groups {
                if g == keep {
                    continue;
                }

                if let Some(members) = rec_groups.get(&g) {
                    for &ty in members {
                        if let Some(def) = self.type_defs.get_mut(&ty) {
                            def.rec_group = keep;
                        }
                    }
                }

                // Drop g from the rec-group set.
                self.rec_groups.remove(&g);
            }
        }

        debug_assert!(
            self.type_defs
                .values()
                .all(|d| self.rec_groups.contains(&d.rec_group)),
            "after rec-group merge, some type_defs still reference removed rec-groups"
        );
    }

    /// Topological sort of rec-groups.
    pub fn sort_rec_groups_topo(
        &self,
        rec_groups: &BTreeMap<RecGroupId, Vec<TypeId>>,
    ) -> Vec<RecGroupId> {
        // deps[g] = set of groups that must come before g
        let mut deps: BTreeMap<RecGroupId, BTreeSet<RecGroupId>> = rec_groups
            .keys()
            .copied()
            .map(|g| (g, BTreeSet::new()))
            .collect();

        for (&g, members) in rec_groups {
            for &id in members {
                let def = &self.type_defs[&id];
                if let Some(st) = def.supertype {
                    let st_group = self.type_defs[&st].rec_group;
                    if st_group != g {
                        deps.get_mut(&g).unwrap().insert(st_group);
                    }
                }
            }
        }

        // indeg[g] = number of prerequisites
        let mut indeg: BTreeMap<RecGroupId, usize> = deps.keys().copied().map(|g| (g, 0)).collect();
        for (&g, ds) in &deps {
            *indeg.get_mut(&g).unwrap() = ds.len();
        }

        //  Prerequisite -> dependents
        let mut users: BTreeMap<RecGroupId, Vec<RecGroupId>> = BTreeMap::new();
        for (&g, ds) in &deps {
            for &d in ds {
                users.entry(d).or_default().push(g);
            }
        }

        // Kahn queue
        let mut q = VecDeque::new();
        for (&g, &d) in &indeg {
            if d == 0 {
                q.push_back(g);
            }
        }

        let mut out = Vec::with_capacity(indeg.len());
        while let Some(g) = q.pop_front() {
            out.push(g);
            if let Some(us) = users.get(&g) {
                for &u in us {
                    let e = indeg.get_mut(&u).unwrap();
                    *e -= 1;
                    if *e == 0 {
                        q.push_back(u);
                    }
                }
            }
        }

        debug_assert_eq!(out.len(), indeg.len(), "cycle in rec-group dependencies");
        out
    }

    /// Topological sort of types by their supertype (supertype before subtype).
    pub fn sort_types_by_supertype(&self) -> Vec<TypeId> {
        #[derive(Copy, Clone, Debug)]
        enum Event {
            Enter,
            Exit,
        }

        let mut stack: Vec<(Event, TypeId)> = self
            .type_defs
            .keys()
            .copied()
            .map(|id| (Event::Enter, id))
            .collect();

        stack.reverse();

        let mut sorted = Vec::with_capacity(self.type_defs.len());
        let mut seen = BTreeSet::<TypeId>::new();

        while let Some((event, id)) = stack.pop() {
            match event {
                Event::Enter => {
                    if seen.insert(id) {
                        stack.push((Event::Exit, id));

                        if let Some(super_id) = self.type_defs[&id].supertype {
                            if !seen.contains(&super_id) {
                                stack.push((Event::Enter, super_id));
                            }
                        }
                    }
                }
                Event::Exit => {
                    sorted.push(id);
                }
            }
        }
        sorted
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

        // Build rec_groups map for cycle detection and merging.
        let mut rec_groups_map: BTreeMap<RecGroupId, Vec<TypeId>> = self
            .rec_groups
            .iter()
            .copied()
            .map(|g| (g, Vec::new()))
            .collect();

        for (id, ty) in self.type_defs.iter() {
            rec_groups_map.entry(ty.rec_group).or_default().push(*id);
        }

        self.merge_rec_groups_via_scc(&rec_groups_map);
        self.break_type_cycles_in_rec_groups();

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
