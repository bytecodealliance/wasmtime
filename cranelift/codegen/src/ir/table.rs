//! Tables.

use crate::ir::immediates::Uimm64;
use crate::ir::{Template, Type};
use core::fmt;

/// Information about a table declaration.
#[derive(Clone)]
pub struct TableData {
    /// Template computing the address of the start of the table.
    pub base_template: Template,

    /// Guaranteed minimum table size in elements. Table accesses before `min_size` don't need
    /// bounds checking.
    pub min_size: Uimm64,

    /// Template computing the current bound of the table, in elements.
    pub bound_template: Template,

    /// The size of a table element, in bytes.
    pub element_size: Uimm64,

    /// The index type for the table.
    pub index_type: Type,
}

impl fmt::Display for TableData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("dynamic")?;
        write!(
            f,
            " {}, min {}, bound {}, element_size {}, index_type {}",
            self.base_template,
            self.min_size,
            self.bound_template,
            self.element_size,
            self.index_type
        )
    }
}
