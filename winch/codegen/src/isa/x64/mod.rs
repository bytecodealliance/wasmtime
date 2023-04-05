use crate::{
    abi::ABI,
    codegen::{CodeGen, CodeGenContext},
};

use crate::frame::{DefinedLocals, Frame};
use crate::isa::x64::masm::MacroAssembler as X64Masm;
use crate::masm::MacroAssembler;
use crate::regalloc::RegAlloc;
use crate::stack::Stack;
use crate::trampoline::Trampoline;
use crate::FuncEnv;
use crate::{
    isa::{Builder, TargetIsa},
    regset::RegSet,
};
use anyhow::Result;
use cranelift_codegen::settings::{self, Flags};
use cranelift_codegen::{isa::x64::settings as x64_settings, Final, MachBufferFinalized};
use cranelift_codegen::{MachTextSectionBuilder, TextSectionBuilder};
use target_lexicon::Triple;
use wasmparser::{FuncType, FuncValidator, FunctionBody, ValidatorResources};

use self::regs::ALL_GPR;

mod abi;
mod address;
mod asm;
mod masm;
// Not all the fpr and gpr constructors are used at the moment;
// in that sense, this directive is a temporary measure to avoid
// dead code warnings.
#[allow(dead_code)]
mod regs;

/// Create an ISA builder.
pub(crate) fn isa_builder(triple: Triple) -> Builder {
    Builder::new(
        triple,
        x64_settings::builder(),
        |triple, shared_flags, settings| {
            // TODO: Once enabling/disabling flags is allowed, and once features like SIMD are supported
            // ensure compatibility between shared flags and ISA flags.
            let isa_flags = x64_settings::Flags::new(&shared_flags, settings);
            let isa = X64::new(triple, shared_flags, isa_flags);
            Ok(Box::new(isa))
        },
    )
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

    fn flags(&self) -> &settings::Flags {
        &self.shared_flags
    }

    fn isa_flags(&self) -> Vec<settings::Value> {
        self.isa_flags.iter().collect()
    }

    fn compile_function(
        &self,
        sig: &FuncType,
        body: &FunctionBody,
        env: &dyn FuncEnv,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<MachBufferFinalized<Final>> {
        let mut body = body.get_binary_reader();
        let mut masm = X64Masm::new(self.shared_flags.clone(), self.isa_flags.clone());
        let stack = Stack::new();
        let abi = abi::X64ABI::default();
        let abi_sig = abi.sig(sig);

        let defined_locals = DefinedLocals::new(&mut body, validator)?;
        let frame = Frame::new(&abi_sig, &defined_locals, &abi)?;
        // TODO Add in floating point bitmask
        let regalloc = RegAlloc::new(RegSet::new(ALL_GPR, 0), regs::scratch());
        let codegen_context = CodeGenContext::new(regalloc, stack, &frame);
        let mut codegen = CodeGen::new(&mut masm, &abi, codegen_context, env, abi_sig);

        codegen.emit(&mut body, validator)?;

        Ok(masm.finalize())
    }

    fn text_section_builder(&self, num_funcs: usize) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<cranelift_codegen::isa::x64::Inst>::new(num_funcs))
    }

    fn function_alignment(&self) -> u32 {
        // See `cranelift_codegen`'s value of this for more information.
        16
    }

    fn host_to_wasm_trampoline(&self, ty: &FuncType) -> Result<MachBufferFinalized<Final>> {
        let abi = abi::X64ABI::default();
        let mut masm = X64Masm::new(self.shared_flags.clone(), self.isa_flags.clone());

        let mut trampoline = Trampoline::new(&mut masm, &abi, regs::scratch(), regs::argv());

        trampoline.emit_host_to_wasm(ty);

        Ok(masm.finalize())
    }
}
