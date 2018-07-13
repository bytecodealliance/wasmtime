//! Global values.

use ir::immediates::Offset32;
use ir::{ExternalName, GlobalValue};
use std::fmt;

/// Information about a global value declaration.
#[derive(Clone)]
pub enum GlobalValueData {
    /// Value is the address of a field in the VM context struct, a constant offset from the VM
    /// context pointer.
    VMContext {
        /// Offset from the `vmctx` pointer.
        offset: Offset32,
    },

    /// Value is pointed to by another global value.
    ///
    /// The `base` global value is assumed to contain a pointer. This global value is computed
    /// by loading from memory at that pointer value, and then adding an offset. The memory must
    /// be accessible, and naturally aligned to hold a pointer value.
    Deref {
        /// The base pointer global value.
        base: GlobalValue,

        /// Byte offset to be added to the loaded value.
        offset: Offset32,
    },

    /// Value is identified by a symbolic name. Cranelift itself does not interpret this name;
    /// it's used by embedders to link with other data structures.
    Sym {
        /// The symbolic name.
        name: ExternalName,

        /// Will this symbol be defined nearby, such that it will always be a certain distance
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
