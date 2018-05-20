//! Tables.

use ir::immediates::Imm64;
use ir::GlobalValue;
use std::fmt;

/// Information about a table declaration.
#[derive(Clone)]
pub struct TableData {
    /// Global value giving the address of the start of the table.
    pub base_gv: GlobalValue,

    /// Guaranteed minimum table size in elements. Table accesses before `min_size` don't need
    /// bounds checking.
    pub min_size: Imm64,

    /// Global value giving the current bound of the table, in elements.
    pub bound_gv: GlobalValue,

    /// The size of a table element, in bytes.
    pub element_size: Imm64,
}

impl fmt::Display for TableData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, min {}, bound {}, element_size {}",
            self.base_gv, self.min_size, self.bound_gv, self.element_size
        )
    }
}
