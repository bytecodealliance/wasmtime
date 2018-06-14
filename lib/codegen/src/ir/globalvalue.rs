//! Global variables.

use ir::immediates::Offset32;
use ir::{ExternalName, GlobalValue};
use std::fmt;

/// Information about a global valueiable declaration.
#[derive(Clone)]
pub enum GlobalValueData {
    /// Variable is part of the VM context struct, it's address is a constant offset from the VM
    /// context pointer.
    VMContext {
        /// Offset from the `vmctx` pointer to this global.
        offset: Offset32,
    },

    /// Variable is part of a struct pointed to by another global valueiable.
    ///
    /// The `base` global valueiable is assumed to contain a pointer to a struct. This global
    /// variable lives at an offset into the struct. The memory must be accessible, and
    /// naturally aligned to hold a pointer value.
    Deref {
        /// The base pointer global valueiable.
        base: GlobalValue,

        /// Byte offset to be added to the pointer loaded from `base`.
        offset: Offset32,
    },

    /// Variable is at an address identified by a symbolic name. Cretonne itself
    /// does not interpret this name; it's used by embedders to link with other
    /// data structures.
    Sym {
        /// The symbolic name.
        name: ExternalName,

        /// Will this variable be defined nearby, such that it will always be a certain distance
        /// away, after linking? If so, references to it can avoid going through a GOT. Note that
        /// symbols meant to be preemptible cannot be colocated.
        colocated: bool,
    },
}

impl GlobalValueData {
    /// Assume that `self` is an `GlobalValueData::Sym` and return its name.
    pub fn symbol_name(&self) -> &ExternalName {
        match *self {
            GlobalValueData::Sym { ref name, .. } => name,
            _ => panic!("only symbols have names"),
        }
    }
}

impl fmt::Display for GlobalValueData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GlobalValueData::VMContext { offset } => write!(f, "vmctx{}", offset),
            GlobalValueData::Deref { base, offset } => write!(f, "deref({}){}", base, offset),
            GlobalValueData::Sym {
                ref name,
                colocated,
            } => {
                if colocated {
                    write!(f, "colocated ")?;
                }
                write!(f, "globalsym {}", name)
            }
        }
    }
}
