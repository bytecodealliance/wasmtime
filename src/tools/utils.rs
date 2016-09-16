//! Utility functions.

use std::path::Path;
use std::fs::File;
use std::io::{Result, Read};

/// Read an entire file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = try!(File::open(path));
    let mut buffer = String::new();
    try!(file.read_to_string(&mut buffer));
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

#[test]
fn test_match_directive() {
    assert_eq!(match_directive("; foo: bar  ", "foo:"), Some("bar"));
    assert_eq!(match_directive(" foo:bar", "foo:"), Some("bar"));
    assert_eq!(match_directive("foo:bar", "foo:"), Some("bar"));
    assert_eq!(match_directive(";x foo: bar", "foo:"), None);
    assert_eq!(match_directive(";;; foo: bar", "foo:"), Some("bar"));
}
