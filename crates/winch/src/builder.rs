use crate::compiler::Compiler;
use anyhow::Result;
use cranelift_codegen::settings;
use std::sync::Arc;
use target_lexicon::Triple;
use wasmtime_environ::{CompilerBuilder, Setting};
use winch_codegen::isa;

/// Compiler builder.
struct Builder {
    /// Target triple.
    triple: Triple,
    /// Shared flags builder.
    shared_flags: settings::Builder,
    /// ISA builder.
    isa_builder: isa::Builder,
}

pub fn builder() -> Box<dyn CompilerBuilder> {
    let triple = Triple::host();
    Box::new(Builder {
        triple: triple.clone(),
        shared_flags: settings::builder(),
        // TODO:
        // Either refactor and re-use `cranelift-native::builder()` or come up with a similar
        // mechanism to lookup the host's architecture ISA and infer native flags.
        isa_builder: isa::lookup(triple).expect("host architecture is not supported"),
    })
}

impl CompilerBuilder for Builder {
    fn triple(&self) -> &target_lexicon::Triple {
        &self.triple
    }

    fn target(&mut self, target: target_lexicon::Triple) -> Result<()> {
        self.triple = target;
        Ok(())
    }

    fn set(&mut self, _name: &str, _val: &str) -> Result<()> {
        Ok(())
    }

    fn enable(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    fn settings(&self) -> Vec<Setting> {
        vec![]
    }

    fn build(&self) -> Result<Box<dyn wasmtime_environ::Compiler>> {
        let flags = settings::Flags::new(self.shared_flags.clone());
        Ok(Box::new(Compiler::new(
            self.isa_builder.clone().build(flags)?,
        )))
    }

    fn enable_incremental_compilation(
        &mut self,
        _cache_store: Arc<dyn wasmtime_environ::CacheStore>,
    ) {
        todo!()
    }
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Builder: {{ triple: {:?} }}", self.triple())
    }
}
