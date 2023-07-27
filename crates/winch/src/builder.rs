use crate::compiler::Compiler;
use anyhow::{bail, Result};
use std::sync::Arc;
use wasmtime_cranelift_shared::isa_builder::IsaBuilder;
use wasmtime_environ::{CompilerBuilder, Setting};
use winch_codegen::{isa, TargetIsa};

/// Compiler builder.
struct Builder {
    inner: IsaBuilder<Result<Box<dyn TargetIsa>>>,
}

pub fn builder() -> Box<dyn CompilerBuilder> {
    Box::new(Builder {
        inner: IsaBuilder::new(|triple| isa::lookup(triple).map_err(|e| e.into())),
    })
}

impl CompilerBuilder for Builder {
    fn triple(&self) -> &target_lexicon::Triple {
        self.inner.triple()
    }

    fn target(&mut self, target: target_lexicon::Triple) -> Result<()> {
        self.inner.target(target)?;
        Ok(())
    }

    fn set(&mut self, name: &str, value: &str) -> Result<()> {
        self.inner.set(name, value)
    }

    fn enable(&mut self, name: &str) -> Result<()> {
        self.inner.enable(name)
    }

    fn settings(&self) -> Vec<Setting> {
        self.inner.settings()
    }

    fn set_tunables(&mut self, tunables: wasmtime_environ::Tunables) -> Result<()> {
        let _ = tunables;
        Ok(())
    }

    fn build(&self) -> Result<Box<dyn wasmtime_environ::Compiler>> {
        let isa = self.inner.build()?;

        Ok(Box::new(Compiler::new(isa)))
    }

    fn enable_incremental_compilation(
        &mut self,
        _cache_store: Arc<dyn wasmtime_environ::CacheStore>,
    ) -> Result<()> {
        bail!("incremental compilation is not supported on this platform");
    }
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Builder: {{ triple: {:?} }}", self.triple())
    }
}
