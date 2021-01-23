//! 32-bit ARM Instruction Set Architecture.

use crate::ir::condcodes::IntCC;
use crate::ir::Function;
use crate::isa::Builder as IsaBuilder;
use crate::machinst::{compile, MachBackend, MachCompileResult, TargetIsaAdapter, VCode};
use crate::result::CodegenResult;
use crate::settings;

use alloc::boxed::Box;
use regalloc::{PrettyPrint, RealRegUniverse};
use target_lexicon::{Architecture, ArmArchitecture, Triple};

// New backend:
mod abi;
mod inst;
mod lower;
mod lower_inst;

use inst::{create_reg_universe, EmitInfo};

/// An ARM32 backend.
pub struct Arm32Backend {
    triple: Triple,
    flags: settings::Flags,
    reg_universe: RealRegUniverse,
}

impl Arm32Backend {
    /// Create a new ARM32 backend with the given (shared) flags.
    pub fn new_with_flags(triple: Triple, flags: settings::Flags) -> Arm32Backend {
        let reg_universe = create_reg_universe();
        Arm32Backend {
            triple,
            flags,
            reg_universe,
        }
    }

    fn compile_vcode(
        &self,
        func: &Function,
        flags: settings::Flags,
    ) -> CodegenResult<VCode<inst::Inst>> {
        // This performs lowering to VCode, register-allocates the code, computes
        // block layout and finalizes branches. The result is ready for binary emission.
        let emit_info = EmitInfo::new(flags.clone());
        let abi = Box::new(abi::Arm32ABICallee::new(func, flags)?);
        compile::compile::<Arm32Backend>(func, self, abi, emit_info)
    }
}

impl MachBackend for Arm32Backend {
    fn compile_function(
        &self,
        func: &Function,
        want_disasm: bool,
    ) -> CodegenResult<MachCompileResult> {
        let flags = self.flags();
        let vcode = self.compile_vcode(func, flags.clone())?;
        let buffer = vcode.emit();
        let frame_size = vcode.frame_size();

        let disasm = if want_disasm {
            Some(vcode.show_rru(Some(&create_reg_universe())))
        } else {
            None
        };

        let buffer = buffer.finish();

        Ok(MachCompileResult {
            buffer,
            frame_size,
            disasm,
            unwind_info: None,
            value_labels_ranges: None,
        })
    }

    fn name(&self) -> &'static str {
        "arm32"
    }

    fn triple(&self) -> Triple {
        self.triple.clone()
    }

    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn reg_universe(&self) -> &RealRegUniverse {
        &self.reg_universe
    }

    fn unsigned_add_overflow_condition(&self) -> IntCC {
        // Carry flag set.
        IntCC::UnsignedGreaterThanOrEqual
    }

    fn unsigned_sub_overflow_condition(&self) -> IntCC {
        // Carry flag clear.
        IntCC::UnsignedLessThan
    }
}

/// Create a new `isa::Builder`.
pub fn isa_builder(triple: Triple) -> IsaBuilder {
    assert!(match triple.architecture {
        Architecture::Arm(ArmArchitecture::Arm)
        | Architecture::Arm(ArmArchitecture::Armv7)
        | Architecture::Arm(ArmArchitecture::Armv6) => true,
        _ => false,
    });
    IsaBuilder {
        triple,
        setup: settings::builder(),
        constructor: |triple, shared_flags, _| {
            let backend = Arm32Backend::new_with_flags(triple, shared_flags);
            Box::new(TargetIsaAdapter::new(backend))
        },
    }
}
