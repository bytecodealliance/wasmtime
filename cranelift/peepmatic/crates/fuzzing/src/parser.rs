//! Utilities for fuzzing our DSL's parser.

use peepmatic::Optimizations;
use std::str;

/// Attempt to parse the given string as if it were a snippet of our DSL.
pub fn parse(data: &[u8]) {
    let source = match str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let buf = match wast::parser::ParseBuffer::new(&source) {
        Ok(buf) => buf,
        Err(_) => return,
    };

    let _ = wast::parser::parse::<Optimizations>(&buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_parse() {
        crate::check(|s: String| parse(s.as_bytes()));
    }
}
