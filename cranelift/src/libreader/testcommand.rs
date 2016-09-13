//! Test commands.
//!
//! A `.cton` file can begin with one or more *test commands* which specify what is to be tested.
//! The general syntax is:
//!
//! <pre>
//! test <i>&lt;command&gt;</i> </i>[options]</i>...
//! </pre>
//!
//! The options are either a single identifier flag, or setting values like `identifier=value`.
//!
//! The parser does not understand the test commands or which options are alid. It simply parses
//! the general format into a `TestCommand` data structure.

use std::fmt::{self, Display, Formatter};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestCommand<'a> {
    pub command: &'a str,
    pub options: Vec<TestOption<'a>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TestOption<'a> {
    Flag(&'a str),
    Value(&'a str, &'a str),
}

impl<'a> TestCommand<'a> {
    pub fn new(s: &'a str) -> TestCommand<'a> {
        let mut parts = s.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        TestCommand {
            command: cmd,
            options: parts.filter(|s| !s.is_empty()).map(TestOption::new).collect(),
        }
    }
}

impl<'a> Display for TestCommand<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.command));
        for opt in &self.options {
            try!(write!(f, " {}", opt));
        }
        writeln!(f, "")
    }
}

impl<'a> TestOption<'a> {
    pub fn new(s: &'a str) -> TestOption<'a> {
        match s.find('=') {
            None => TestOption::Flag(s),
            Some(p) => TestOption::Value(&s[0..p], &s[p + 1..]),
        }
    }
}

impl<'a> Display for TestOption<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            TestOption::Flag(s) => write!(f, "{}", s),
            TestOption::Value(s, v) => write!(f, "{}={}", s, v),
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
        assert_eq!(&TestCommand::new("cat  one=4   two t").to_string(),
                   "cat one=4 two t\n");
    }
}
