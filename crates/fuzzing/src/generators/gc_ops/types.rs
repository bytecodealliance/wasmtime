//! Types for the `gc` operations.

use crate::generators::gc_ops::limits::GcOpsLimits;
use crate::generators::gc_ops::ops::GcOp;
use serde::{Deserialize, Serialize};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

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
    pub(crate) composite_type: CompositeType,
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
    pub fn insert_empty_struct(&mut self, id: TypeId, group: RecGroupId) {
        self.rec_groups
            .get_mut(&group)
            .expect("rec group must exist")
            .insert(id);
        self.type_defs.insert(
            id,
            SubType {
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

    /// Fix up the types to ensure they are within the limits.
    pub fn fixup(&mut self, limits: &GcOpsLimits) {
        let max_rec_groups =
            usize::try_from(limits.max_rec_groups).expect("max_rec_groups is too large");
        let max_types = usize::try_from(limits.max_types).expect("max_types is too large");

        // 1. Trim excess types and remove them from rec group member sets, i.e.
        while self.type_defs.len() > max_types {
            if let Some((tid, _)) = self.type_defs.pop_last() {
                for members in self.rec_groups.values_mut() {
                    members.remove(&tid);
                }
            }
        }

        // 2. Drop dangling member set entries that reference types that do not exist.
        for members in self.rec_groups.values_mut() {
            members.retain(|tid| self.type_defs.contains_key(tid));
        }

        // 3. Trim excess rec groups and collect their members as orphans.
        let mut rec_group_orphans = BTreeSet::new();
        while self.rec_groups.len() > max_rec_groups {
            if let Some((_gid, members)) = self.rec_groups.pop_last() {
                rec_group_orphans.extend(members);
            }
        }

        // 4. Find corruption orphans that are not in any group.
        let mut all_members = BTreeSet::new();
        for members in self.rec_groups.values() {
            all_members.extend(members.iter().copied());
        }

        // Exclude rec_group_orphans that are already accounted for in the step 3.
        let corruption_orphans: BTreeSet<TypeId> = self
            .type_defs
            .keys()
            .filter(|tid| !all_members.contains(tid) && !rec_group_orphans.contains(tid))
            .copied()
            .collect();

        // 5. Adopt into the first rec group: corruption orphans first,
        //    then rec group orphans. Both are already within max_types from step 1.
        if let Some(gid) = self.rec_groups.keys().next().copied() {
            let members = self.rec_groups.get_mut(&gid).unwrap();
            members.extend(corruption_orphans);
            members.extend(rec_group_orphans);
        } else {
            // No rec groups at all — drop everything.
            for tid in corruption_orphans.iter().chain(rec_group_orphans.iter()) {
                self.type_defs.remove(tid);
            }
        }

        debug_assert!(self.is_well_formed(limits));
    }

    /// Check if the types are well-formed and within configured limits, i.e.
    /// rec/type counts are within limits,
    /// every type belongs to exactly one rec group,
    /// and every rec group member must exist in type_defs.
    fn is_well_formed(&self, limits: &GcOpsLimits) -> bool {
        if self.rec_groups.len()
            > usize::try_from(limits.max_rec_groups).expect("max_rec_groups is too large")
        {
            return false;
        }
        if self.type_defs.len() > usize::try_from(limits.max_types).expect("max_types is too large")
        {
            return false;
        }
        let mut all = BTreeSet::new();
        for members in self.rec_groups.values() {
            for tid in members {
                if !self.type_defs.contains_key(tid) {
                    return false;
                }
                if !all.insert(*tid) {
                    return false;
                }
            }
        }
        self.type_defs.keys().all(|tid| all.contains(tid))
    }
}

/// Tracks the required operand type on the abstract value stack.
#[derive(Copy, Clone, Debug)]
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
    ) {
        let mut result_types = Vec::new();
        match req {
            None => {
                if stack.is_empty() {
                    Self::emit(GcOp::NullExtern, stack, out, num_types, &mut result_types);
                }
                stack.pop();
            }
            Some(Self::ExternRef) => match stack.last() {
                Some(Self::ExternRef) => {
                    stack.pop();
                }
                _ => {
                    Self::emit(GcOp::NullExtern, stack, out, num_types, &mut result_types);
                    stack.pop();
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

    /// Emit an opcode and update the stack.
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

    /// Clamp a type index to the number of types.
    fn clamp(t: u32, n: u32) -> u32 {
        if n == 0 { 0 } else { t % n }
    }
}
