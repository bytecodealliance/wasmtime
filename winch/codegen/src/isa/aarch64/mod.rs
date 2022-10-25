use crate::isa::TargetIsa;
use anyhow::Result;
use target_lexicon::Triple;
use wasmparser::{FuncType, FuncValidator, FunctionBody, ValidatorResources};

mod abi;
mod masm;
mod regs;

/// Create an ISA from the given triple.
pub(crate) fn isa_from(triple: Triple) -> Aarch64 {
    Aarch64::new(triple)
}

pub(crate) struct Aarch64 {
    triple: Triple,
}

impl Aarch64 {
    pub fn new(triple: Triple) -> Self {
        Self { triple }
    }
}

impl TargetIsa for Aarch64 {
    fn name(&self) -> &'static str {
        "aarch64"
    }

    fn triple(&self) -> &Triple {
        &self.triple
    }

    fn compile_function(
        &self,
        _sig: &FuncType,
        mut _body: FunctionBody,
        mut _validator: FuncValidator<ValidatorResources>,
    ) -> Result<Vec<String>> {
        todo!()
    }
}
