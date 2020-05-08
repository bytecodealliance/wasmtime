//! X86_64-bit Instruction Set Architecture.

use alloc::boxed::Box;

use regalloc::RealRegUniverse;
use target_lexicon::Triple;

use crate::ir::Function;
use crate::isa::Builder as IsaBuilder;
use crate::machinst::pretty_print::ShowWithRRU;
use crate::machinst::{compile, MachBackend, MachCompileResult, TargetIsaAdapter, VCode};
use crate::result::CodegenResult;
use crate::settings::{self, Flags};

use crate::isa::x64::inst::regs::create_reg_universe_systemv;

mod abi;
mod inst;
mod lower;

/// An X64 backend.
pub(crate) struct X64Backend {
    triple: Triple,
    flags: Flags,
    reg_universe: RealRegUniverse,
}

impl X64Backend {
    /// Create a new X64 backend with the given (shared) flags.
    fn new_with_flags(triple: Triple, flags: Flags) -> Self {
        let reg_universe = create_reg_universe_systemv(&flags);
        Self {
            triple,
            flags,
            reg_universe,
        }
    }

    fn compile_vcode(&self, func: &Function, flags: Flags) -> CodegenResult<VCode<inst::Inst>> {
        // This performs lowering to VCode, register-allocates the code, computes
        // block layout and finalizes branches. The result is ready for binary emission.
        let abi = Box::new(abi::X64ABIBody::new(&func, flags));
        compile::compile::<Self>(&func, self, abi)
    }
}

impl MachBackend for X64Backend {
    fn compile_function(
        &self,
        func: &Function,
        want_disasm: bool,
    ) -> CodegenResult<MachCompileResult> {
        let flags = self.flags();
        let vcode = self.compile_vcode(func, flags.clone())?;
        let sections = vcode.emit();
        let frame_size = vcode.frame_size();

        let disasm = if want_disasm {
            Some(vcode.show_rru(Some(&create_reg_universe_systemv(flags))))
        } else {
            None
        };

        Ok(MachCompileResult {
            sections,
            frame_size,
            disasm,
        })
    }

    fn flags(&self) -> &Flags {
        &self.flags
    }

    fn name(&self) -> &'static str {
        "x64"
    }

    fn triple(&self) -> Triple {
        self.triple.clone()
    }

    fn reg_universe(&self) -> &RealRegUniverse {
        &self.reg_universe
    }
}

/// Create a new `isa::Builder`.
pub(crate) fn isa_builder(triple: Triple) -> IsaBuilder {
    IsaBuilder {
        triple,
        setup: settings::builder(),
        constructor: |triple: Triple, flags: Flags, _arch_flag_builder: settings::Builder| {
            let backend = X64Backend::new_with_flags(triple, flags);
            Box::new(TargetIsaAdapter::new(backend))
        },
    }
}
