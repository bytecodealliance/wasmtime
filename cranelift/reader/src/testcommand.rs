//! Test commands.
//!
//! A `.clif` file can begin with one or more *test commands* which specify what is to be tested.
//! The general syntax is:
//!
//! <pre>
//! test <i>&lt;command&gt;</i> <i>[options]</i>...
//! </pre>
//!
//! The options are either a single identifier flag, or setting values like `identifier=value`.
//!
//! The parser does not understand the test commands or which options are valid. It simply parses
//! the general format into a `TestCommand` data structure.

use std::fmt::{self, Display, Formatter};

/// A command appearing in a test file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestCommand<'a> {
    /// The command name as a string.
    pub command: &'a str,
    /// The options following the command name.
    pub options: Vec<TestOption<'a>>,
}

/// An option on a test command.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TestOption<'a> {
    /// Single identifier flag: `foo`.
    Flag(&'a str),
    /// A value assigned to an identifier: `foo=bar`.
    Value(&'a str, &'a str),
}

impl<'a> TestCommand<'a> {
    /// Create a new TestCommand by parsing `s`.
    /// The returned command contains references into `s`.
    pub fn new(s: &'a str) -> Self {
        let mut parts = s.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        Self {
            command: cmd,
            options: parts
                .filter(|s| !s.is_empty())
                .map(TestOption::new)
                .collect(),
        }
    }
}

impl<'a> Display for TestCommand<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.command)?;
        for opt in &self.options {
            write!(f, " {opt}")?;
        }
        writeln!(f)
    }
}

impl<'a> TestOption<'a> {
    /// Create a new TestOption by parsing `s`.
    /// The returned option contains references into `s`.
    pub fn new(s: &'a str) -> Self {
        match s.find('=') {
            None => TestOption::Flag(s),
            Some(p) => TestOption::Value(&s[0..p], &s[p + 1..]),
        }
    }
}

impl<'a> Display for TestOption<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            TestOption::Flag(s) => write!(f, "{s}"),
            TestOption::Value(s, v) => write!(f, "{s}={v}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_option() {
        assert_eq!(TestOption::new(""), TestOption::Flag(""));
        assert_eq!(TestOption::new("foo"), TestOption::Flag("foo"));
        assert_eq!(TestOption::new("foo=bar"), TestOption::Value("foo", "bar"));
    }

    #[test]
    fn parse_command() {
        assert_eq!(&TestCommand::new("").to_string(), "\n");
        assert_eq!(&TestCommand::new("cat").to_string(), "cat\n");
        assert_eq!(&TestCommand::new("cat  ").to_string(), "cat\n");
        assert_eq!(&TestCommand::new("cat  1  ").to_string(), "cat 1\n");
        assert_eq!(
            &TestCommand::new("cat  one=4   two t").to_string(),
            "cat one=4 two t\n"
        );
    }
}
