use crate::isa::x64::masm::MacroAssembler;
use crate::{compilation_env::CompilationEnv, isa::TargetIsa, regset::RegSet};
use anyhow::Result;
use target_lexicon::Triple;
use wasmtime_environ::{FunctionBodyData, WasmFuncType};

use self::regs::ALL_GPR;

mod abi;
mod masm;
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

    fn compile_function(&self, sig: &WasmFuncType, body: FunctionBodyData) -> Result<Vec<String>> {
        // Temporarily returns a '&static str
        // TODO
        // 1. Derive calling convention (panic if unsupported)
        // 2. Check for multi-value returns
        //     * Panic if using multi-value (support for multi-value will be added in a follow-up)
        // 3. Check for usage of ref types
        //     * Panic if using ref types
        // 4. Create a compilation_env and call `emit`
        let FunctionBodyData {
            validator,
            mut body,
        } = body;

        let abi = abi::X64ABI::default();
        let mut validator = validator.into_validator(Default::default());
        let regset = RegSet::new(ALL_GPR, 0);
        let masm = MacroAssembler::new();
        let mut env = CompilationEnv::new(sig, &mut body, &mut validator, abi, masm, regset)?;

        env.emit()
    }
}
