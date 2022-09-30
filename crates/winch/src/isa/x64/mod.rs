use crate::{compilation_env::CompilationEnv, isa::TargetIsa};
use anyhow::Result;
use target_lexicon::Triple;
use wasmtime_environ::{FunctionBodyData, WasmFuncType};

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

    fn compile_function(
        &self,
        sig: &WasmFuncType,
        body: &mut FunctionBodyData,
    ) -> Result<Vec<String>> {
        // Temporarily returns a '&static str
        // TODO
        // 1. Derive calling convention (panic if unsupported)
        // 2. Check for multi-value returns
        //     * Panic if using multi-value (support for multi-value will be added in a follow-up)
        // 3. Check for usage of ref types
        //     * Panic if using ref types
        // 4. Create a compilation_env and call `emit`
        let abi = abi::X64ABI::default();
        let asm = masm::MacroAssembler::default();
        let mut env = CompilationEnv::new(sig, body, abi, asm)?;

        env.emit()
    }
}
