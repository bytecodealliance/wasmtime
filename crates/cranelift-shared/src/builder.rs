//! Implementation of a "compiler builder" for cranelift
//!
//! This module contains the implementation of how Cranelift is configured, as
//! well as providing a function to return the default configuration to build.

use anyhow::Result;
use cranelift_codegen::isa::IsaBuilder;
use cranelift_codegen::settings::{self, Configurable, SetError};
use std::fmt;
use std::sync::Arc;
use target_lexicon::Triple;
use wasmtime_environ::{CacheStore, CompilerBuilder, Setting, SettingKind};

pub struct Builder<T> {
    pub flags: settings::Builder,
    pub isa_flags: IsaBuilder<T>,
    pub linkopts: LinkOptions,
    pub cache_store: Option<Arc<dyn CacheStore>>,
    pub lookup: fn(Triple) -> Result<IsaBuilder<T>>,
    pub build: fn(T, &Self) -> Result<Box<dyn wasmtime_environ::Compiler>>,
}

#[derive(Clone, Default)]
pub struct LinkOptions {
    /// A debug-only setting used to synthetically insert 0-byte padding between
    /// compiled functions to simulate huge compiled artifacts and exercise
    /// logic related to jump veneers.
    pub padding_between_functions: usize,

    /// A debug-only setting used to force inter-function calls in a wasm module
    /// to always go through "jump veneers" which are typically only generated
    /// when functions are very far from each other.
    pub force_jump_veneers: bool,
}

pub fn builder<T>(
    lookup: fn(Triple) -> Result<IsaBuilder<T>>,
    build: fn(T, &Builder<T>) -> Result<Box<dyn wasmtime_environ::Compiler>>,
) -> Box<dyn CompilerBuilder>
where
    T: 'static,
{
    let mut flags = settings::builder();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    flags
        .enable("avoid_div_traps")
        .expect("should be valid flag");

    // We don't use probestack as a stack limit mechanism
    flags
        .set("enable_probestack", "false")
        .expect("should be valid flag");

    Box::new(Builder {
        flags,
        isa_flags: lookup(Triple::host()).expect("host machine is not a supported target"),
        linkopts: LinkOptions::default(),
        cache_store: None,
        lookup,
        build,
    })
}

impl<T> CompilerBuilder for Builder<T> {
    fn triple(&self) -> &target_lexicon::Triple {
        self.isa_flags.triple()
    }

    fn target(&mut self, target: Triple) -> Result<()> {
        self.isa_flags = (self.lookup)(target)?;
        Ok(())
    }

    fn set(&mut self, name: &str, value: &str) -> Result<()> {
        // Special wasmtime-cranelift-only settings first
        if name == "wasmtime_linkopt_padding_between_functions" {
            self.linkopts.padding_between_functions = value.parse()?;
            return Ok(());
        }
        if name == "wasmtime_linkopt_force_jump_veneer" {
            self.linkopts.force_jump_veneers = value.parse()?;
            return Ok(());
        }

        // ... then forward this to Cranelift
        if let Err(err) = self.flags.set(name, value) {
            match err {
                SetError::BadName(_) => {
                    // Try the target-specific flags.
                    self.isa_flags.set(name, value)?;
                }
                _ => return Err(err.into()),
            }
        }
        Ok(())
    }

    fn enable(&mut self, name: &str) -> Result<()> {
        if let Err(err) = self.flags.enable(name) {
            match err {
                SetError::BadName(_) => {
                    // Try the target-specific flags.
                    self.isa_flags.enable(name)?;
                }
                _ => return Err(err.into()),
            }
        }
        Ok(())
    }

    fn build(&self) -> Result<Box<dyn wasmtime_environ::Compiler>> {
        let isa = self
            .isa_flags
            .finish(settings::Flags::new(self.flags.clone()));
        (self.build)(isa, self)
    }

    fn settings(&self) -> Vec<Setting> {
        self.isa_flags
            .iter()
            .map(|s| Setting {
                description: s.description,
                name: s.name,
                values: s.values,
                kind: match s.kind {
                    settings::SettingKind::Preset => SettingKind::Preset,
                    settings::SettingKind::Enum => SettingKind::Enum,
                    settings::SettingKind::Num => SettingKind::Num,
                    settings::SettingKind::Bool => SettingKind::Bool,
                },
            })
            .collect()
    }

    fn enable_incremental_compilation(
        &mut self,
        cache_store: Arc<dyn wasmtime_environ::CacheStore>,
    ) {
        self.cache_store = Some(cache_store);
    }
}

impl<T> fmt::Debug for Builder<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Builder")
            .field(
                "flags",
                &settings::Flags::new(self.flags.clone()).to_string(),
            )
            .finish()
    }
}
