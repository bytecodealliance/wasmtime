//! Adapter for a `MachBackend` to implement the `TargetIsa` trait.

use crate::ir;
use crate::isa::TargetIsa;
use crate::machinst::*;
use crate::settings::{self, Flags};

#[cfg(feature = "unwind")]
use crate::isa::unwind::systemv::RegisterMappingError;

use std::fmt;
use target_lexicon::Triple;

/// A wrapper around a `MachBackend` that provides a `TargetIsa` impl.
pub struct TargetIsaAdapter {
    backend: Box<dyn MachBackend + Send + Sync + 'static>,
}

impl TargetIsaAdapter {
    /// Create a new `TargetIsa` wrapper around a `MachBackend`.
    pub fn new<B: MachBackend + Send + Sync + 'static>(backend: B) -> TargetIsaAdapter {
        TargetIsaAdapter {
            backend: Box::new(backend),
        }
    }
}

impl fmt::Display for TargetIsaAdapter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MachBackend")
            .field("name", &self.backend.name())
            .field("triple", &self.backend.triple())
            .field("flags", &format!("{}", self.backend.flags()))
            .finish()
    }
}

impl TargetIsa for TargetIsaAdapter {
    fn name(&self) -> &'static str {
        self.backend.name()
    }

    fn triple(&self) -> &Triple {
        self.backend.triple()
    }

    fn flags(&self) -> &Flags {
        self.backend.flags()
    }

    fn isa_flags(&self) -> Vec<settings::Value> {
        self.backend.isa_flags()
    }

    fn compile_function(
        &self,
        func: &Function,
        want_disasm: bool,
    ) -> CodegenResult<MachCompileResult> {
        self.backend.compile_function(func, want_disasm)
    }

    fn get_mach_backend(&self) -> &dyn MachBackend {
        &*self.backend
    }

    fn unsigned_add_overflow_condition(&self) -> ir::condcodes::IntCC {
        self.backend.unsigned_add_overflow_condition()
    }

    #[cfg(feature = "unwind")]
    fn create_systemv_cie(&self) -> Option<gimli::write::CommonInformationEntry> {
        self.backend.create_systemv_cie()
    }

    #[cfg(feature = "unwind")]
    fn map_regalloc_reg_to_dwarf(&self, r: Reg) -> Result<u16, RegisterMappingError> {
        self.backend.map_reg_to_dwarf(r)
    }
}
