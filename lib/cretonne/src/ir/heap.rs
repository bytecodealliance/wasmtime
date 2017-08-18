//! Heaps.

use ir::immediates::Imm64;
use ir::GlobalVar;
use std::fmt;

/// Information about a heap declaration.
#[derive(Clone)]
pub struct HeapData {
    /// Method for determining the heap base address.
    pub base: HeapBase,

    /// Guaranteed minimum heap size in bytes. Heap accesses before `min_size` don't need bounds
    /// checking.
    pub min_size: Imm64,

    /// Size in bytes of the guard pages following the heap.
    pub guard_size: Imm64,

    /// Heap style, with additional style-specific info.
    pub style: HeapStyle,
}

/// Method for determining the base address of a heap.
#[derive(Clone)]
pub enum HeapBase {
    /// The heap base lives in a reserved register.
    ReservedReg,

    /// The heap base is in a global variable.
    GlobalVar(GlobalVar),
}

/// Style of heap including style-specific information.
#[derive(Clone)]
pub enum HeapStyle {
    /// A dynamic heap can be relocated to a different base address when it is grown.
    Dynamic {
        /// Global variable holding the current bound of the heap in bytes.
        bound_gv: GlobalVar,
    },

    /// A static heap has a fixed base address and a number of not-yet-allocated pages before the
    /// guard pages.
    Static {
        /// Heap bound in bytes. The guard pages are allocated after the bound.
        bound: Imm64,
    },
}

impl fmt::Display for HeapData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self.style {
                           HeapStyle::Dynamic { .. } => "dynamic",
                           HeapStyle::Static { .. } => "static",
                       })?;

        match self.base {
            HeapBase::ReservedReg => write!(f, " reserved_reg")?,
            HeapBase::GlobalVar(gv) => write!(f, " {}", gv)?,
        }

        write!(f, ", min {}", self.min_size)?;
        match self.style {
            HeapStyle::Dynamic { bound_gv } => write!(f, ", bound {}", bound_gv)?,
            HeapStyle::Static { bound } => write!(f, ", bound {}", bound)?,
        }
        write!(f, ", guard {}", self.guard_size)
    }
}
