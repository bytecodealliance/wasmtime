use self::regs::{scratch, ALL_GPR};
use crate::{
    abi::ABI,
    codegen::{CodeGen, CodeGenContext},
    frame::{DefinedLocals, Frame},
    isa::{Builder, TargetIsa},
    masm::MacroAssembler,
    regalloc::RegAlloc,
    regset::RegSet,
    stack::Stack,
    FuncEnv,
};
use anyhow::Result;
use cranelift_codegen::settings::{self, Flags};
use cranelift_codegen::{isa::aarch64::settings as aarch64_settings, Final, MachBufferFinalized};
use cranelift_codegen::{MachTextSectionBuilder, TextSectionBuilder};
use masm::MacroAssembler as Aarch64Masm;
use target_lexicon::Triple;
use wasmparser::{FuncType, FuncValidator, FunctionBody, ValidatorResources};

mod abi;
mod address;
mod asm;
mod masm;
mod regs;

/// Create an ISA from the given triple.
pub(crate) fn isa_builder(triple: Triple) -> Builder {
    Builder::new(
        triple,
        aarch64_settings::builder(),
        |triple, shared_flags, settings| {
            let isa_flags = aarch64_settings::Flags::new(&shared_flags, settings);
            let isa = Aarch64::new(triple, shared_flags, isa_flags);
            Ok(Box::new(isa))
        },
    )
}

/// Aarch64 ISA.
// Until Aarch64 emission is supported.
#[allow(dead_code)]
pub(crate) struct Aarch64 {
    /// The target triple.
    triple: Triple,
    /// ISA specific flags.
    isa_flags: aarch64_settings::Flags,
    /// Shared flags.
    shared_flags: Flags,
}

impl Aarch64 {
    /// Create an Aarch64 ISA.
    pub fn new(triple: Triple, shared_flags: Flags, isa_flags: aarch64_settings::Flags) -> Self {
        Self {
            isa_flags,
            shared_flags,
            triple,
        }
    }
}

impl TargetIsa for Aarch64 {
    fn name(&self) -> &'static str {
        "aarch64"
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

    fn is_branch_protection_enabled(&self) -> bool {
        self.isa_flags.use_bti()
    }

    fn compile_function(
        &self,
        sig: &FuncType,
        body: &FunctionBody,
        env: &dyn FuncEnv,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<MachBufferFinalized<Final>> {
        let mut body = body.get_binary_reader();
        let mut masm = Aarch64Masm::new(self.shared_flags.clone());
        let stack = Stack::new();
        let abi = abi::Aarch64ABI::default();
        let abi_sig = abi.sig(sig);

        let defined_locals = DefinedLocals::new(&mut body, validator)?;
        let frame = Frame::new(&abi_sig, &defined_locals, &abi)?;
        // TODO: Add floating point bitmask
        let regalloc = RegAlloc::new(RegSet::new(ALL_GPR, 0), scratch());
        let codegen_context = CodeGenContext::new(regalloc, stack, &frame);
        let mut codegen = CodeGen::new(&mut masm, &abi, codegen_context, env, abi_sig);

        codegen.emit(&mut body, validator)?;
        Ok(masm.finalize())
    }

    fn text_section_builder(&self, num_funcs: usize) -> Box<dyn TextSectionBuilder> {
        Box::new(MachTextSectionBuilder::<
            cranelift_codegen::isa::aarch64::inst::Inst,
        >::new(num_funcs))
    }

    fn function_alignment(&self) -> u32 {
        // See `cranelift_codegen::isa::TargetIsa::function_alignment`.
        32
    }

    fn host_to_wasm_trampoline(&self, _ty: &FuncType) -> Result<MachBufferFinalized<Final>> {
        todo!()
    }
}
