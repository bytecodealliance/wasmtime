//! Utility functions.

use cretonne::isa;
use cretonne::isa::TargetIsa;
use cretonne::settings::{self, FlagsOrIsa};
use cton_reader::{parse_options, Location};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Read an entire file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Read an entire file into a vector of bytes.
pub fn read_to_end<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Like `FlagsOrIsa`, but holds ownership.
pub enum OwnedFlagsOrIsa {
    Flags(settings::Flags),
    Isa(Box<TargetIsa>),
}

impl OwnedFlagsOrIsa {
    /// Produce a FlagsOrIsa reference.
    pub fn as_fisa(&self) -> FlagsOrIsa {
        match *self {
            OwnedFlagsOrIsa::Flags(ref flags) => FlagsOrIsa::from(flags),
            OwnedFlagsOrIsa::Isa(ref isa) => FlagsOrIsa::from(&**isa),
        }
    }
}

/// Parse "set" and "isa" commands.
pub fn parse_sets_and_isa(flag_set: &[String], flag_isa: &str) -> Result<OwnedFlagsOrIsa, String> {
    let mut flag_builder = settings::builder();
    parse_options(
        flag_set.iter().map(|x| x.as_str()),
        &mut flag_builder,
        &Location { line_number: 0 },
    ).map_err(|err| err.to_string())?;

    let mut words = flag_isa.trim().split_whitespace();
    // Look for `isa foo`.
    if let Some(isa_name) = words.next() {
        let mut isa_builder = isa::lookup(isa_name).map_err(|err| match err {
            isa::LookupError::Unknown => format!("unknown ISA '{}'", isa_name),
            isa::LookupError::Unsupported => format!("support for ISA '{}' not enabled", isa_name),
        })?;
        // Apply the ISA-specific settings to `isa_builder`.
        parse_options(words, &mut isa_builder, &Location { line_number: 0 })
            .map_err(|err| err.to_string())?;

        Ok(OwnedFlagsOrIsa::Isa(
            isa_builder.finish(settings::Flags::new(&flag_builder)),
        ))
    } else {
        Ok(OwnedFlagsOrIsa::Flags(settings::Flags::new(&flag_builder)))
    }
}
