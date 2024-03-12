use crate::compiler::Compiler;
use anyhow::{bail, Result};
use std::sync::Arc;
use target_lexicon::Triple;
use wasmtime_cranelift_shared::isa_builder::IsaBuilder;
use wasmtime_environ::{CompilerBuilder, Setting};
use winch_codegen::{isa, TargetIsa};

/// Compiler builder.
struct Builder {
    inner: IsaBuilder<Result<Box<dyn TargetIsa>>>,
    cranelift: Box<dyn CompilerBuilder>,
}

pub fn builder(triple: Option<Triple>) -> Result<Box<dyn CompilerBuilder>> {
    let inner = IsaBuilder::new(triple.clone(), |triple| {
        isa::lookup(triple).map_err(|e| e.into())
    })?;
    let cranelift = wasmtime_cranelift::builder(triple)?;
    Ok(Box::new(Builder { inner, cranelift }))
}

impl CompilerBuilder for Builder {
    fn triple(&self) -> &target_lexicon::Triple {
        self.inner.triple()
    }

    fn target(&mut self, target: target_lexicon::Triple) -> Result<()> {
        self.inner.target(target.clone())?;
        self.cranelift.target(target)?;
        Ok(())
    }

    fn set(&mut self, name: &str, value: &str) -> Result<()> {
        self.inner.set(name, value)?;
        self.cranelift.set(name, value)?;
        Ok(())
    }

    fn enable(&mut self, name: &str) -> Result<()> {
        self.inner.enable(name)?;
        self.cranelift.enable(name)?;
        Ok(())
    }

    fn settings(&self) -> Vec<Setting> {
        self.inner.settings()
    }

    fn set_tunables(&mut self, tunables: wasmtime_environ::Tunables) -> Result<()> {
        assert!(tunables.winch_callable);
        self.cranelift.set_tunables(tunables)?;
        Ok(())
    }

    fn build(&self) -> Result<Box<dyn wasmtime_environ::Compiler>> {
        let isa = self.inner.build()?;
        let cranelift = self.cranelift.build()?;
        Ok(Box::new(Compiler::new(isa, cranelift)))
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
