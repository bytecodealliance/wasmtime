//! Table commands.
//!
//! Functions in a `.clif` file can have *table commands* appended that control the tables allocated
//! by the `test run` and `test interpret` infrastructure.
//!
//! The general syntax is:
//! - `; table: entry_size=n, count=m`
//!
//!
//! `entry_size=n` indicates the size of each entry (in bytes) on the table.
//!
//! `count=m` indicates the number of entries allocated on the table.

use cranelift_codegen::ir::immediates::Uimm64;
use std::fmt::{self, Display, Formatter};

/// A table command appearing in a test file.
///
/// For parsing, see `Parser::parse_table_command`
#[derive(PartialEq, Debug, Clone)]
pub struct TableCommand {
    /// Size of each entry on the table.
    pub entry_size: Uimm64,
    /// Number of entries on the table.
    pub entry_count: Uimm64,
    /// Offset of the table pointer from the vmctx base
    ///
    /// This is done for verification purposes only
    pub ptr_offset: Option<Uimm64>,
    /// Offset of the table pointer from the vmctx base
    ///
    /// This is done for verification purposes only
    pub bound_offset: Option<Uimm64>,
}

impl Display for TableCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "table: entry_size={}, count={}",
            self.entry_size, self.entry_count
        )?;

        if let Some(offset) = self.ptr_offset {
            write!(f, ", ptr=vmctx+{}", offset)?
        }

        if let Some(offset) = self.bound_offset {
            write!(f, ", bound=vmctx+{}", offset)?
        }

        Ok(())
    }
}
