//! Types for the `gc` operations.

use crate::generators::gc_ops::limits::GcOpsLimits;
use crate::generators::gc_ops::ops::GcOp;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

    /// Returns a fresh rec-group id one past the current maximum.
    pub fn next_rec_group_id(&self) -> RecGroupId {
        RecGroupId(
            self.rec_groups
                .iter()
                .next_back()
                .map(|g| g.0)
                .unwrap_or(0)
                .saturating_add(1),
        )
    }

    /// Returns a fresh type id one past the current maximum.
    pub fn next_type_id(&self) -> TypeId {
        TypeId(
            self.type_defs
                .keys()
                .next_back()
                .map(|id| id.0)
                .unwrap_or(0)
                .saturating_add(1),
        )
    }

    /// Insert a rec-group id. Returns true if newly inserted, false if it already existed.
    pub fn insert_rec_group(&mut self, id: RecGroupId) -> bool {
        self.rec_groups.insert(id)
    }

    /// Insert a rec-group id.
    pub fn insert_empty_struct(&mut self, id: TypeId, group: RecGroupId) {
        self.type_defs.insert(
            id,
            SubType {
                rec_group: group,
                composite_type: CompositeType::Struct(StructType::default()),
            },
        );
    }

    /// Removes any entries beyond the given limit.
    pub fn fixup(&mut self, limits: &GcOpsLimits) {
        while self.rec_groups.len() > limits.max_rec_groups as usize {
            self.rec_groups.pop_last();
        }

        // Drop any types whose rec-group has been trimmed out.
        self.type_defs
            .retain(|_, ty| self.rec_groups.contains(&ty.rec_group));

        // Then enforce the max types limit.
        while self.type_defs.len() > limits.max_types as usize {
            self.type_defs.pop_last();
        }

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
                            if num_types == 0 {
                                unreachable!(
                                    "typed struct requirement with num_types == 0; op should have been removed"
                                );
                            } else {
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
