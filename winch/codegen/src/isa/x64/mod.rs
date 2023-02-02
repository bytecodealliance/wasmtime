use crate::abi::ABI;
use crate::codegen::{CodeGen, CodeGenContext};
use crate::frame::Frame;
use crate::isa::x64::masm::MacroAssembler as X64Masm;
use crate::masm::MacroAssembler;
use crate::regalloc::RegAlloc;
use crate::stack::Stack;
use crate::{
    isa::{Builder, TargetIsa},
    regset::RegSet,
};
use anyhow::Result;
use cranelift_codegen::{
    isa::x64::settings as x64_settings, settings::Flags, Final, MachBufferFinalized,
};
use target_lexicon::Triple;
use wasmparser::{FuncType, FuncValidator, FunctionBody, ValidatorResources};

use self::regs::ALL_GPR;

mod abi;
mod asm;
mod masm;
// Not all the fpr and gpr constructors are used at the moment;
// in that sense, this directive is a temporary measure to avoid
// dead code warnings.
#[allow(dead_code)]
mod regs;

/// Create an ISA builder.
pub(crate) fn isa_builder(triple: Triple) -> Builder {
    Builder {
        triple,
        settings: x64_settings::builder(),
        constructor: |triple, shared_flags, settings| {
            // TODO: Once enabling/disabling flags is allowed, and once features like SIMD are supported
            // ensure compatibility between shared flags and ISA flags.
            let isa_flags = x64_settings::Flags::new(&shared_flags, settings);
            let isa = X64::new(triple, shared_flags, isa_flags);
            Ok(Box::new(isa))
        },
    }
}

/// x64 ISA.
pub(crate) struct X64 {
    /// The target triple.
    triple: Triple,
    /// ISA specific flags.
    isa_flags: x64_settings::Flags,
    /// Shared flags.
    shared_flags: Flags,
}

impl X64 {
    /// Create a x64 ISA.
    pub fn new(triple: Triple, shared_flags: Flags, isa_flags: x64_settings::Flags) -> Self {
        Self {
            isa_flags,
            shared_flags,
            triple,
        }
    }
}

impl TargetIsa for X64 {
    fn name(&self) -> &'static str {
        "x64"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn compile_function(
        &self,
        sig: &FuncType,
        body: &FunctionBody,
        mut validator: FuncValidator<ValidatorResources>,
    ) -> Result<MachBufferFinalized<Final>> {
        let mut body = body.get_binary_reader();
        let mut masm = X64Masm::new(self.shared_flags.clone(), self.isa_flags.clone());
        let stack = Stack::new();
        let abi = abi::X64ABI::default();
        let abi_sig = abi.sig(sig);
        let frame = Frame::new(&abi_sig, &mut body, &mut validator, &abi)?;
        // TODO Add in floating point bitmask
        let regalloc = RegAlloc::new(RegSet::new(ALL_GPR, 0), regs::scratch());
        let codegen_context = CodeGenContext::new(&mut masm, stack, &frame);
        let mut codegen = CodeGen::new::<abi::X64ABI>(codegen_context, abi_sig, regalloc);

        codegen.emit(&mut body, validator)?;

        Ok(masm.finalize())
    }
}
