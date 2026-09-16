//! Limits for the `gc` operations.

use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Range for the number of parameters.
pub const NUM_PARAMS_RANGE: RangeInclusive<u32> = 0..=10;
/// Range for the maximum number of types.
pub const MAX_TYPES_RANGE: RangeInclusive<u32> = 0..=32;
/// Range for the number of globals.
pub const NUM_GLOBALS_RANGE: RangeInclusive<u32> = 0..=10;
/// Range for the table size.
pub const TABLE_SIZE_RANGE: RangeInclusive<u32> = 0..=100;
/// Range for the maximum number of rec groups.
pub const MAX_REC_GROUPS_RANGE: RangeInclusive<u32> = 0..=10;
/// Range for the maximum number of fields per struct type.
pub const MAX_FIELDS_RANGE: RangeInclusive<u32> = 0..=8;
/// Range for the length of created arrays.
pub const ARRAY_LENGTH_RANGE: RangeInclusive<u32> = 1..=16;
/// Maximum number of fields that can be inlined in a struct type.
pub const MAX_INLINE_CONSTRUCTION: u32 = 8;

/// Limits controlling the structure of a generated Wasm module.
#[derive(Clone, Debug, Serialize, Deserialize, mutatis::Mutate)]
pub struct GcOpsLimits {
    #[mutatis(default_mutate)]
    pub(crate) num_params: u32,
    #[mutatis(default_mutate)]
    pub(crate) num_globals: u32,
    #[mutatis(default_mutate)]
    pub(crate) table_size: u32,
    #[mutatis(default_mutate)]
    pub(crate) max_rec_groups: u32,
    #[mutatis(default_mutate)]
    pub(crate) max_types: u32,
    #[mutatis(default_mutate)]
    pub(crate) max_fields: u32,
    #[mutatis(default_mutate)]
    pub(crate) array_length: u32,
}

impl Default for GcOpsLimits {
    fn default() -> Self {
        Self {
            num_params: 5,
            num_globals: 5,
            table_size: 5,
            max_rec_groups: 5,
            max_types: 5,
            max_fields: 5,
            array_length: 5,
        }
    }
}

impl GcOpsLimits {
    /// Fixup the limits to ensure they are within the valid range.
    pub(crate) fn fixup(&mut self) {
        let Self {
            num_params,
            num_globals,
            table_size,
            max_rec_groups,
            max_types,
            max_fields,
            array_length,
        } = self;

        let fixup = |limit: &mut u32, range: RangeInclusive<u32>| {
            if !range.contains(limit) {
                let (start, end) = (*range.start(), *range.end());
                *limit = start + *limit % (end - start + 1);
            }
        };
        fixup(table_size, TABLE_SIZE_RANGE);
        fixup(num_params, NUM_PARAMS_RANGE);
        fixup(num_globals, NUM_GLOBALS_RANGE);
        fixup(max_rec_groups, MAX_REC_GROUPS_RANGE);
        fixup(max_types, MAX_TYPES_RANGE);
        fixup(max_fields, MAX_FIELDS_RANGE);
        fixup(array_length, ARRAY_LENGTH_RANGE);
    }
}
