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
/// Maximum number of operations.
pub const MAX_OPS: usize = 100;

/// Limits controlling the structure of a generated Wasm module.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GcOpsLimits {
    pub(crate) num_params: u32,
    pub(crate) num_globals: u32,
    pub(crate) table_size: u32,
    pub(crate) max_rec_groups: u32,
    pub(crate) max_types: u32,
}
impl GcOpsLimits {
    /// Fixup the limits to ensure they are within the valid range.
    pub(crate) fn fixup(&mut self) {
        // NB: Exhaustively match so that we remember to fixup any other new
        // limits we add in the future.
        let Self {
            num_params,
            num_globals,
            table_size,
            max_rec_groups,
            max_types,
        } = self;

        let clamp = |limit: &mut u32, range: RangeInclusive<u32>| {
            *limit = (*limit).clamp(*range.start(), *range.end())
        };
        clamp(table_size, TABLE_SIZE_RANGE);
        clamp(num_params, NUM_PARAMS_RANGE);
        clamp(num_globals, NUM_GLOBALS_RANGE);
        clamp(max_rec_groups, MAX_REC_GROUPS_RANGE);
        clamp(max_types, MAX_TYPES_RANGE);
    }
}
