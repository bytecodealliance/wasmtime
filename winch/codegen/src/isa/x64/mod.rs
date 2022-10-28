use crate::abi::ABI;
use crate::codegen::{CodeGen, CodeGenContext};
use crate::frame::Frame;
use crate::isa::x64::masm::MacroAssembler;
use crate::regalloc::RegAlloc;
use crate::stack::Stack;
use crate::{isa::TargetIsa, regset::RegSet};
use anyhow::Result;
use target_lexicon::Triple;
use wasmparser::{FuncType, FuncValidator, FunctionBody, ValidatorResources};

use self::regs::ALL_GPR;

mod abi;
mod masm;
// Not all the fpr and gpr constructors are used at the moment;
// in that sense, this directive is a temporary measure to avoid
// dead code warnings.
#[allow(dead_code)]
mod regs;

/// Create an ISA from the given triple.
pub(crate) fn isa_from(triple: Triple) -> X64 {
    X64::new(triple)
}

pub(crate) struct X64 {
    triple: Triple,
}

impl X64 {
    pub fn new(triple: Triple) -> Self {
        Self { triple }
    }
}

impl TargetIsa for X64 {
    fn name(&self) -> &'static str {
        "x64"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    // Temporarily returns a Vec<String>
    fn compile_function(
        &self,
        sig: &FuncType,
        mut body: FunctionBody,
        mut validator: FuncValidator<ValidatorResources>,
    ) -> Result<Vec<String>> {
        let masm = MacroAssembler::new();
        let stack = Stack::new();
        let abi = abi::X64ABI::default();
        let abi_sig = abi.sig(sig);
        let frame = Frame::new(&abi_sig, &mut body, &mut validator, &abi)?;
        // TODO Add in floating point bitmask
        let regalloc = RegAlloc::new(RegSet::new(ALL_GPR, 0), regs::scratch());
        let codegen_context = CodeGenContext::new(masm, stack, &frame);
        let mut codegen =
            CodeGen::new::<abi::X64ABI>(codegen_context, abi_sig, body, validator, regalloc);

        codegen.emit()
    }
}
