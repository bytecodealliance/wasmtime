use crate::abi::ABI;
use crate::codegen::{CodeGen, CodeGenContext};
use crate::frame::Frame;
use crate::isa::x64::masm::MacroAssembler;
use crate::regalloc::RegAlloc;
use crate::stack::Stack;
use crate::{isa::TargetIsa, regset::RegSet};
use anyhow::Result;
use target_lexicon::Triple;
use wasmtime_environ::{FunctionBodyData, WasmFuncType};

use self::regs::ALL_GPR;

mod abi;
mod masm;
// Temporarily disable dead code warnings
// for unused registers
#[allow(dead_code)]
mod regs;

/// Create an ISA from the given triple
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
    fn compile_function(&self, sig: &WasmFuncType, body: FunctionBodyData) -> Result<Vec<String>> {
        let FunctionBodyData {
            validator,
            mut body,
        } = body;

        let masm = MacroAssembler::new();
        let stack = Stack::new();
        let abi = abi::X64ABI::default();
        let abi_sig = abi.sig(sig);
        let mut validator = validator.into_validator(Default::default());
        let frame = Frame::new(&abi_sig, &mut body, &mut validator, &abi)?;
        // TODO Add in floating point bitmask
        let regalloc = RegAlloc::new(RegSet::new(ALL_GPR, 0), regs::scratch());
        let codegen_context = CodeGenContext::new(masm, stack, &frame);
        let mut codegen = CodeGen::new(
            codegen_context,
            abi,
            abi_sig,
            &mut body,
            &mut validator,
            regalloc,
        );

        codegen.emit()
    }
}
