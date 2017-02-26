//! Function names.
//!
//! The name of a function doesn't have any meaning to Cretonne which compiles functions
//! independently.

use std::fmt::{self, Write};
use std::ascii::AsciiExt;

/// The name of a function can be any UTF-8 string.
///
/// Function names are mostly a testing and debugging tool.
/// In particular, `.cton` files use function names to identify functions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionName(String);

impl FunctionName {
    /// Create new function name equal to `s`.
    pub fn new<S: Into<String>>(s: S) -> FunctionName {
        FunctionName(s.into())
    }
}

fn is_id_start(c: char) -> bool {
    c.is_ascii() && (c == '_' || c.is_alphabetic())
}

fn is_id_continue(c: char) -> bool {
    c.is_ascii() && (c == '_' || c.is_alphanumeric())
}

// The function name may need quotes if it doesn't parse as an identifier.
fn needs_quotes(name: &str) -> bool {
    let mut iter = name.chars();
    if let Some(ch) = iter.next() {
        !is_id_start(ch) || !iter.all(is_id_continue)
    } else {
        // A blank function name needs quotes.
        true
    }
}

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if needs_quotes(&self.0) {
            f.write_char('"')?;
            for c in self.0.chars().flat_map(char::escape_default) {
                f.write_char(c)?;
            }
            f.write_char('"')
        } else {
            f.write_str(&self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{needs_quotes, FunctionName};

    #[test]
    fn quoting() {
        assert_eq!(needs_quotes(""), true);
        assert_eq!(needs_quotes("x"), false);
        assert_eq!(needs_quotes(" "), true);
        assert_eq!(needs_quotes("0"), true);
        assert_eq!(needs_quotes("x0"), false);
    }

    #[test]
    fn escaping() {
        assert_eq!(FunctionName::new("").to_string(), "\"\"");
        assert_eq!(FunctionName::new("x").to_string(), "x");
        assert_eq!(FunctionName::new(" ").to_string(), "\" \"");
        assert_eq!(FunctionName::new(" \n").to_string(), "\" \\n\"");
        assert_eq!(FunctionName::new("a\u{1000}v").to_string(),
                   "\"a\\u{1000}v\"");
    }
}
