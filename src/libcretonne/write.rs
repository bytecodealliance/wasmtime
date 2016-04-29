//! Converting Cretonne IL to text.
//!
//! The `write` module provides the `write_function` function which converts an IL `Function` to an
//! equivalent textual representation. This textual representation can be read back by the
//! `cretonne-reader` crate.

use std::io::{self, Write};
use repr::Function;

pub type Result = io::Result<()>;

/// Write `func` to `w` as equivalent text.
pub fn write_function(w: &mut Write, func: &Function) -> Result {
    try!(write_spec(w, func));
    try!(writeln!(w, " {{"));
    try!(write_preamble(w, func));
    writeln!(w, "}}")
}

/// Convert `func` to a string.
pub fn function_to_string(func: &Function) -> String {
    let mut buffer: Vec<u8> = Vec::new();
    // Any errors here would be out-of-memory, which should not happen with normal functions.
    write_function(&mut buffer, func).unwrap();
    // A UTF-8 conversion error is a real bug.
    String::from_utf8(buffer).unwrap()
}

// ====--------------------------------------------------------------------------------------====//
//
// Function spec.
//
// ====--------------------------------------------------------------------------------------====//

// The function name may need quotes if it doesn't parse as an identifier.
fn needs_quotes(name: &str) -> bool {
    let mut iter = name.chars();
    if let Some(ch) = iter.next() {
        !ch.is_alphabetic() || !iter.all(char::is_alphanumeric)
    } else {
        // A blank function name needs quotes.
        true
    }
}

// Use Rust's escape_default which provides a few simple \t \r \n \' \" \\ escapes and uses
// \u{xxxx} for anything else outside the ASCII printable range.
fn escaped(name: &str) -> String {
    name.chars().flat_map(char::escape_default).collect()
}

fn write_spec(w: &mut Write, func: &Function) -> Result {
    let sig = func.own_signature();
    if !needs_quotes(&func.name) {
        write!(w, "function {}{}", func.name, sig)
    } else {
        write!(w, "function \"{}\" {}", escaped(&func.name), sig)
    }
}

fn write_preamble(w: &mut Write, func: &Function) -> Result {
    let mut any = false;

    for ss in func.stack_slot_iter() {
        any = true;
        try!(writeln!(w, "    {} = {}", ss, func[ss]));
    }

    // Put a blank line after the preamble unless it was empty.
    if any {
        writeln!(w, "")
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{needs_quotes, escaped};
    use repr::{Function, StackSlotData};

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
        assert_eq!(escaped(""), "");
        assert_eq!(escaped("x"), "x");
        assert_eq!(escaped(" "), " ");
        assert_eq!(escaped(" \n"), " \\n");
        assert_eq!(escaped("a\u{1000}v"), "a\\u{1000}v");
    }

    #[test]
    fn basic() {
        let mut f = Function::new();
        assert_eq!(function_to_string(&f), "function \"\" () {\n}\n");

        f.name.push_str("foo");
        assert_eq!(function_to_string(&f), "function foo() {\n}\n");

        f.make_stack_slot(StackSlotData::new(4));
        assert_eq!(function_to_string(&f),
                   "function foo() {\n    ss0 = stack_slot 4\n\n}\n");
    }
}
