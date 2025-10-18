//! Types for the `gc` operations.

use crate::generators::gc_ops::limits::GcOpsLimits;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// RecGroup ID struct definition.
#[derive(
    Debug, Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct RecGroupId(pub(crate) u32);

/// TypeID struct definition.
#[derive(Debug, Clone, Eq, PartialOrd, PartialEq, Ord, Hash, Default, Serialize, Deserialize)]
pub struct TypeId(pub(crate) u32);

/// StructType definition
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StructType {
    // Empty for now; fields will come in a future PR.
}

/// CompsiteType definition
#[derive(Debug, Serialize, Deserialize)]
pub enum CompositeType {
    /// Struct Type definition
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

    /// Insert a rec-group id. Returns true if newly inserted, false if it already existed.
    pub fn insert_rec_group(&mut self, id: RecGroupId) -> bool {
        self.rec_groups.insert(id)
    }

    ///  Insert a rec-group id.
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
