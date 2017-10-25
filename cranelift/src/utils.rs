//! Utility functions.

use cretonne::ir::entities::AnyEntity;
use cretonne::{ir, verifier};
use cretonne::result::CtonError;
use cretonne::isa::TargetIsa;
use cretonne::settings::{self, FlagsOrIsa};
use cretonne::isa;
use cton_reader::{parse_options, Location};
use std::fmt::Write;
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

/// Look for a directive in a comment string.
/// The directive is of the form "foo:" and should follow the leading `;` in the comment:
///
/// ; dominates: ebb3 ebb4
///
/// Return the comment text following the directive.
pub fn match_directive<'a>(comment: &'a str, directive: &str) -> Option<&'a str> {
    assert!(
        directive.ends_with(':'),
        "Directive must include trailing colon"
    );
    let text = comment.trim_left_matches(';').trim_left();
    if text.starts_with(directive) {
        Some(text[directive.len()..].trim())
    } else {
        None
    }
}

/// Pretty-print a verifier error.
pub fn pretty_verifier_error(
    func: &ir::Function,
    isa: Option<&TargetIsa>,
    err: verifier::Error,
) -> String {
    let mut msg = err.to_string();
    match err.location {
        AnyEntity::Inst(inst) => {
            write!(msg, "\n{}: {}\n\n", inst, func.dfg.display_inst(inst, isa)).unwrap()
        }
        _ => msg.push('\n'),
    }
    write!(msg, "{}", func.display(isa)).unwrap();
    msg
}

/// Pretty-print a Cretonne error.
pub fn pretty_error(func: &ir::Function, isa: Option<&TargetIsa>, err: CtonError) -> String {
    if let CtonError::Verifier(e) = err {
        pretty_verifier_error(func, isa, e)
    } else {
        err.to_string()
    }
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
pub fn parse_sets_and_isa(
    flag_set: Vec<String>,
    flag_isa: String,
) -> Result<OwnedFlagsOrIsa, String> {
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

#[test]
fn test_match_directive() {
    assert_eq!(match_directive("; foo: bar  ", "foo:"), Some("bar"));
    assert_eq!(match_directive(" foo:bar", "foo:"), Some("bar"));
    assert_eq!(match_directive("foo:bar", "foo:"), Some("bar"));
    assert_eq!(match_directive(";x foo: bar", "foo:"), None);
    assert_eq!(match_directive(";;; foo: bar", "foo:"), Some("bar"));
}
