//! Implementation of a "compiler builder" for cranelift
//!
//! This module contains the implementation of how Cranelift is configured, as
//! well as providing a function to return the default configuration to build.

use anyhow::Result;
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable, SetError};
use std::fmt;
use wasmtime_environ::{CompilerBuilder, Setting, SettingKind};

#[derive(Clone)]
struct Builder {
    flags: settings::Builder,
    isa_flags: isa::Builder,

    // A debug-only setting used to synthetically insert 0-byte padding between
    // compiled functions to simulate huge compiled artifacts and exercise logic
    // related to jump veneers.
    padding_between_functions: usize,
}

pub fn builder() -> Box<dyn CompilerBuilder> {
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
        isa_flags: cranelift_native::builder().expect("host machine is not a supported target"),
        padding_between_functions: 0,
    })
}

impl CompilerBuilder for Builder {
    fn triple(&self) -> &target_lexicon::Triple {
        self.isa_flags.triple()
    }

    fn clone(&self) -> Box<dyn CompilerBuilder> {
        Box::new(Clone::clone(self))
    }

    fn target(&mut self, target: target_lexicon::Triple) -> Result<()> {
        self.isa_flags = isa::lookup(target)?;
        Ok(())
    }

    fn set(&mut self, name: &str, value: &str) -> Result<()> {
        // Special wasmtime-cranelift-only setting.
        if name == "padding_between_functions" {
            self.padding_between_functions = value.parse()?;
            return Ok(());
        }

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

    fn build(&self) -> Box<dyn wasmtime_environ::Compiler> {
        let isa = self
            .isa_flags
            .clone()
            .finish(settings::Flags::new(self.flags.clone()));
        Box::new(crate::compiler::Compiler::new(
            isa,
            self.padding_between_functions,
        ))
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
}

impl fmt::Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Builder")
            .field(
                "flags",
                &settings::Flags::new(self.flags.clone()).to_string(),
            )
            .finish()
    }
}
