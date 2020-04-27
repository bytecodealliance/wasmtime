//! Utility functions.

use cranelift_codegen::isa;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, FlagsOrIsa};
use cranelift_reader::{parse_options, Location, ParseError, ParseOptionError};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;
use target_lexicon::Triple;

/// Read an entire file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut buffer = String::new();
    if path.as_ref() == Path::new("-") {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();
        stdin.read_to_string(&mut buffer)?;
    } else {
        let mut file = File::open(path)?;
        file.read_to_string(&mut buffer)?;
    }
    Ok(buffer)
}

/// Like `FlagsOrIsa`, but holds ownership.
pub enum OwnedFlagsOrIsa {
    Flags(settings::Flags),
    Isa(Box<dyn TargetIsa>),
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
pub fn parse_sets_and_triple(
    flag_set: &[String],
    flag_triple: &str,
) -> Result<OwnedFlagsOrIsa, String> {
    let mut flag_builder = settings::builder();

    // Collect unknown system-wide settings, so we can try to parse them as target specific
    // settings, if a target is defined.
    let mut unknown_settings = Vec::new();
    match parse_options(
        flag_set.iter().map(|x| x.as_str()),
        &mut flag_builder,
        Location { line_number: 0 },
    ) {
        Err(ParseOptionError::UnknownFlag { name, .. }) => {
            unknown_settings.push(name);
        }
        Err(ParseOptionError::Generic(err)) => return Err(err.to_string()),
        Ok(()) => {}
    }

    let mut words = flag_triple.trim().split_whitespace();
    // Look for `target foo`.
    if let Some(triple_name) = words.next() {
        let triple = match Triple::from_str(triple_name) {
            Ok(triple) => triple,
            Err(parse_error) => return Err(parse_error.to_string()),
        };

        let mut isa_builder = isa::lookup(triple).map_err(|err| match err {
            isa::LookupError::SupportDisabled => {
                format!("support for triple '{}' is disabled", triple_name)
            }
            isa::LookupError::Unsupported => format!(
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
        .map_err(|err| ParseError::from(err).to_string())?;

        // Apply the ISA-specific settings to `isa_builder`.
        parse_options(words, &mut isa_builder, Location { line_number: 0 })
            .map_err(|err| ParseError::from(err).to_string())?;

        Ok(OwnedFlagsOrIsa::Isa(
            isa_builder.finish(settings::Flags::new(flag_builder)),
        ))
    } else {
        if !unknown_settings.is_empty() {
            return Err(format!(
                "unknown settings: '{}'",
                unknown_settings.join("', '")
            ));
        }
        Ok(OwnedFlagsOrIsa::Flags(settings::Flags::new(flag_builder)))
    }
}
