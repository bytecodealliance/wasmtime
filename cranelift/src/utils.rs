//! Utility functions.

use cretonne::ir::entities::AnyEntity;
use cretonne::{ir, verifier, write_function};
use cretonne::result::CtonError;
use std::fmt::Write;
use std::fs::File;
use std::io::{Result, Read};
use std::path::Path;

/// Read an entire file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Look for a directive in a comment string.
/// The directive is of the form "foo:" and should follow the leading `;` in the comment:
///
/// ; dominates: ebb3 ebb4
///
/// Return the comment text following the directive.
pub fn match_directive<'a>(comment: &'a str, directive: &str) -> Option<&'a str> {
    assert!(directive.ends_with(':'),
            "Directive must include trailing colon");
    let text = comment.trim_left_matches(';').trim_left();
    if text.starts_with(directive) {
        Some(text[directive.len()..].trim())
    } else {
        None
    }
}

/// Pretty-print a verifier error.
pub fn pretty_verifier_error(func: &ir::Function, err: verifier::Error) -> String {
    let mut msg = err.to_string();
    match err.location {
        AnyEntity::Inst(inst) => {
            write!(msg, "\n{}: {}\n\n", inst, func.dfg.display_inst(inst)).unwrap()
        }
        _ => msg.push('\n'),
    }
    write_function(&mut msg, func, None).unwrap();
    msg
}

/// Pretty-print a Cretonne error.
pub fn pretty_error(func: &ir::Function, err: CtonError) -> String {
    if let CtonError::Verifier(e) = err {
        pretty_verifier_error(func, e)
    } else {
        err.to_string()
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
