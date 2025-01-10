//! Cranelift file reader library.
//!
//! The `cranelift_reader` library supports reading .clif files. This functionality is needed for
//! testing Cranelift, but is not essential for a JIT compiler.

#![deny(missing_docs)]

pub use crate::error::{Location, ParseError, ParseResult};
pub use crate::isaspec::{IsaSpec, ParseOptionError, parse_option, parse_options};
pub use crate::parser::{ParseOptions, parse_functions, parse_run_command, parse_test};
pub use crate::run_command::{Comparison, Invocation, RunCommand};
pub use crate::sourcemap::SourceMap;
pub use crate::testcommand::{TestCommand, TestOption};
pub use crate::testfile::{Comment, Details, Feature, TestFile};

mod error;
mod isaspec;
mod lexer;
mod parser;
mod run_command;
mod sourcemap;
mod testcommand;
mod testfile;

use anyhow::{Error, Result};
use cranelift_codegen::isa::{self, OwnedTargetIsa};
use cranelift_codegen::settings::{self, FlagsOrIsa};
use std::str::FromStr;
use target_lexicon::Triple;

/// Like `FlagsOrIsa`, but holds ownership.
#[allow(missing_docs, reason = "self-describing variants")]
pub enum OwnedFlagsOrIsa {
    Flags(settings::Flags),
    Isa(OwnedTargetIsa),
}

impl OwnedFlagsOrIsa {
    /// Produce a FlagsOrIsa reference.
    pub fn as_fisa(&self) -> FlagsOrIsa {
        match *self {
            Self::Flags(ref flags) => FlagsOrIsa::from(flags),
            Self::Isa(ref isa) => FlagsOrIsa::from(&**isa),
        }
    }
}

/// Parse "set" and "triple" commands.
pub fn parse_sets_and_triple(flag_set: &[String], flag_triple: &str) -> Result<OwnedFlagsOrIsa> {
    let mut flag_builder = settings::builder();

    // Collect unknown system-wide settings, so we can try to parse them as target specific
    // settings, if a target is defined.
    let mut unknown_settings = Vec::new();
    for flag in flag_set {
        match parse_option(flag, &mut flag_builder, Location { line_number: 0 }) {
            Err(ParseOptionError::UnknownFlag { name, .. }) => {
                unknown_settings.push(name);
            }
            Err(ParseOptionError::UnknownValue { name, value, .. }) => {
                unknown_settings.push(format!("{name}={value}"));
            }
            Err(ParseOptionError::Generic(err)) => return Err(err.into()),
            Ok(()) => {}
        }
    }

    let mut words = flag_triple.trim().split_whitespace();
    // Look for `target foo`.
    if let Some(triple_name) = words.next() {
        let triple = match Triple::from_str(triple_name) {
            Ok(triple) => triple,
            Err(parse_error) => return Err(Error::from(parse_error)),
        };

        let mut isa_builder = isa::lookup(triple).map_err(|err| match err {
            isa::LookupError::SupportDisabled => {
                anyhow::anyhow!("support for triple '{}' is disabled", triple_name)
            }
            isa::LookupError::Unsupported => anyhow::anyhow!(
                "support for triple '{}' is not implemented yet",
                triple_name
            ),
        })?;

        // Try to parse system-wide unknown settings as target-specific settings.
        parse_options(
            unknown_settings.iter().map(|x| x.as_str()),
            &mut isa_builder,
            Location { line_number: 0 },
        )
        .map_err(ParseError::from)?;

        // Apply the ISA-specific settings to `isa_builder`.
        parse_options(words, &mut isa_builder, Location { line_number: 0 })
            .map_err(ParseError::from)?;

        Ok(OwnedFlagsOrIsa::Isa(
            isa_builder.finish(settings::Flags::new(flag_builder))?,
        ))
    } else {
        if !unknown_settings.is_empty() {
            anyhow::bail!("unknown settings: '{}'", unknown_settings.join("', '"));
        }
        Ok(OwnedFlagsOrIsa::Flags(settings::Flags::new(flag_builder)))
    }
}
