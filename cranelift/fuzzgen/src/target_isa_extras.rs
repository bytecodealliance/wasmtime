use cranelift::prelude::isa::TargetIsa;
use target_lexicon::Architecture;

pub trait TargetIsaExtras {
    fn supports_simd(&self) -> bool;
}

impl TargetIsaExtras for &dyn TargetIsa {
    fn supports_simd(&self) -> bool {
        match self.triple().architecture {
            // RISC-V only supports SIMD with the V extension.
            Architecture::Riscv64(_) => self
                .isa_flags()
                .iter()
                .find(|f| f.name == "has_v")
                .and_then(|f| f.as_bool())
                .unwrap_or(false),
            _ => true,
        }
    }
}
