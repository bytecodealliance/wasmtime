use crate::isa::{Builder, TargetIsa};
use anyhow::Result;
use cranelift_codegen::{
    isa::aarch64::settings as aarch64_settings, settings::Flags, Final, MachBufferFinalized,
};
use target_lexicon::Triple;
use wasmparser::{FuncType, FuncValidator, FunctionBody, ValidatorResources};

mod abi;
mod masm;
mod regs;

/// Create an ISA from the given triple.
pub(crate) fn isa_builder(triple: Triple) -> Builder {
    Builder {
        triple,
        settings: aarch64_settings::builder(),
        constructor: |triple, shared_flags, settings| {
            let isa_flags = aarch64_settings::Flags::new(&shared_flags, settings);
            let isa = Aarch64::new(triple, shared_flags, isa_flags);
            Ok(Box::new(isa))
        },
    }
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
    /// Create a Aarch64 ISA.
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

    fn compile_function(
        &self,
        _sig: &FuncType,
        _body: &FunctionBody,
        mut _validator: FuncValidator<ValidatorResources>,
    ) -> Result<MachBufferFinalized<Final>> {
        todo!()
    }
}
