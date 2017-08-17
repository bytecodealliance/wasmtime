//! Global variables.

use ir::GlobalVar;
use ir::immediates::Offset32;
use std::fmt;

/// Information about a global variable declaration.
#[derive(Clone)]
pub enum GlobalVarData {
    /// Variable is part of the VM context struct, it's address is a constant offset from the VM
    /// context pointer.
    VmCtx {
        /// Offset from the `vmctx` pointer to this global.
        offset: Offset32,
    },

    /// Variable is part of a struct pointed to by another global variable.
    ///
    /// The `base` global variable is assumed to contain a pointer to a struct. This global
    /// variable lives at an offset into the struct.
    Deref {
        /// The base pointer global variable.
        base: GlobalVar,

        /// Byte offset to be added to the pointer loaded from `base`.
        offset: Offset32,
    },
}

impl fmt::Display for GlobalVarData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &GlobalVarData::VmCtx { offset } => write!(f, "vmctx{}", offset),
            &GlobalVarData::Deref { base, offset } => write!(f, "deref({}){}", base, offset),
        }
    }
}
