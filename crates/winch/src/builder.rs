use crate::compiler::Compiler;
use anyhow::Result;
use std::sync::Arc;
use target_lexicon::Triple;
use wasmtime_environ::{CompilerBuilder, Setting};
use winch_codegen::isa;

struct Builder {
    triple: Triple,
}

pub fn builder() -> Box<dyn CompilerBuilder> {
    Box::new(Builder {
        triple: Triple::host(),
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
        let isa = isa::lookup(self.triple.clone())?;
        Ok(Box::new(Compiler::new(isa)))
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
