//! Source code generator.
//!
//! The `srcgen` module contains generic helper routines and classes for
//! generating source code.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path;

use error;

static SHIFTWIDTH: usize = 4;

pub struct Formatter {
    indent: usize,
    lines: Vec<String>,
}

impl Formatter {
    /// Source code formatter class. Used to collect source code to be written
    /// to a file, and keep track of indentation.
    pub fn new() -> Self {
        Self {
            indent: 0,
            lines: Vec::new(),
        }
    }

    /// Increase current indentation level by one.
    pub fn indent_push(&mut self) {
        self.indent += 1;
    }

    /// Decrease indentation by one level.
    pub fn indent_pop(&mut self) {
        assert!(self.indent > 0, "Already at top level indentation");
        self.indent -= 1;
    }

    pub fn indent<T, F: FnOnce(&mut Formatter) -> T>(&mut self, f: F) -> T {
        self.indent_push();
        let ret = f(self);
        self.indent_pop();
        ret
    }

    /// Get the current whitespace indentation in the form of a String.
    fn get_indent(&self) -> String {
        if self.indent == 0 {
            String::new()
        } else {
            format!("{:-1$}", " ", self.indent * SHIFTWIDTH)
        }
    }

    /// Get a string containing whitespace outdented one level. Used for
    /// lines of code that are inside a single indented block.
    fn _get_outdent(&mut self) -> String {
        self.indent_push();
        let s = self.get_indent();
        self.indent_pop();
        s
    }

    /// Add an indented line.
    pub fn line(&mut self, contents: &str) {
        let indented_line = format!("{}{}\n", self.get_indent(), contents);
        self.lines.push(indented_line);
    }

    /// Emit a line outdented one level.
    pub fn _outdented_line(&mut self, s: &str) {
        let new_line = format!("{}{}", self._get_outdent(), s);
        self.lines.push(new_line);
    }

    /// Write `self.lines` to a file.
    pub fn update_file(&self, filename: &str, directory: &str) -> Result<(), error::Error> {
        #[cfg(target_family = "windows")]
        let path_str = format!("{}\\{}", directory, filename);
        #[cfg(not(target_family = "windows"))]
        let path_str = format!("{}/{}", directory, filename);

        let path = path::Path::new(&path_str);
        let mut f = fs::File::create(path)?;

        for l in self.lines.iter().map(|l| l.as_bytes()) {
            f.write_all(l)?;
        }

        Ok(())
    }

    /// Add one or more lines after stripping common indentation.
    pub fn _multi_line(&mut self, s: &str) {
        parse_multiline(s).into_iter().for_each(|l| self.line(&l));
    }

    /// Add a comment line.
    pub fn _comment(&mut self, s: &str) {
        let commented_line = format!("// {}", s);
        self.line(&commented_line);
    }

    /// Add a (multi-line) documentation comment.
    pub fn doc_comment(&mut self, contents: &str) {
        parse_multiline(contents)
            .iter()
            .map(|l| format!("/// {}", l))
            .for_each(|s| self.line(s.as_str()));
    }

    /// Add a match expression.
    fn _add_match(&mut self, _m: &_Match) {
        unimplemented!();
    }
}

/// Compute the indentation of s, or None of an empty line.
fn _indent(s: &str) -> Option<usize> {
    if s.is_empty() {
        None
    } else {
        let t = s.trim_left();
        Some(s.len() - t.len())
    }
}

/// Given a multi-line string, split it into a sequence of lines after
/// stripping a common indentation. This is useful for strings defined with
/// doc strings.
fn parse_multiline(s: &str) -> Vec<String> {
    // Convert tabs into spaces.
    let expanded_tab = format!("{:-1$}", " ", SHIFTWIDTH);
    let lines: Vec<String> = s.lines().map(|l| l.replace("\t", &expanded_tab)).collect();

    // Determine minimum indentation, ignoring the first line.
    let indent = lines
        .iter()
        .skip(1)
        .map(|l| l.len() - l.trim_left().len())
        .min();

    // Strip off leading blank lines.
    let mut lines_iter = lines.iter().skip_while(|l| l.is_empty());
    let mut trimmed = Vec::with_capacity(lines.len());

    // Remove indentation (first line is special)
    if let Some(s) = lines_iter.next().map(|l| l.trim()).map(|l| l.to_string()) {
        trimmed.push(s);
    }

    // Remove trailing whitespace from other lines.
    let mut other_lines = if let Some(indent) = indent {
        lines_iter
            .map(|l| &l[indent..])
            .map(|l| l.trim_right())
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
    } else {
        lines_iter
            .map(|l| l.trim_right())
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
    };

    trimmed.append(&mut other_lines);

    // Strip off trailing blank lines.
    while let Some(s) = trimmed.pop() {
        if s.is_empty() {
            continue;
        } else {
            trimmed.push(s);
            break;
        }
    }

    trimmed
}

/// Match formatting class.
///
/// Match objects collect all the information needed to emit a Rust `match`
/// expression, automatically deduplicating overlapping identical arms.
///
/// Note that this class is ignorant of Rust types, and considers two fields
/// with the same name to be equivalent. A BTreeMap is used to represent the
/// arms in order to make the order deterministic.
struct _Match<'a> {
    _expr: &'a str,
    arms: BTreeMap<(Vec<&'a str>, &'a str), HashSet<&'a str>>,
}

impl<'a> _Match<'a> {
    /// Create a new match statement on `expr`.
    fn _new(expr: &'a str) -> Self {
        Self {
            _expr: expr,
            arms: BTreeMap::new(),
        }
    }

    /// Add an arm to the Match statement.
    fn _arm(&mut self, name: &'a str, fields: Vec<&'a str>, body: &'a str) {
        // let key = (fields, body);
        let match_arm = self.arms.entry((fields, body)).or_insert_with(HashSet::new);
        match_arm.insert(name);
    }
}

#[cfg(test)]
mod srcgen_tests {
    use super::_Match;
    use super::parse_multiline;
    use super::Formatter;

    #[test]
    fn adding_arms_works() {
        let mut m = _Match::_new("x");
        m._arm("Orange", vec!["a", "b"], "some body");
        m._arm("Yellow", vec!["a", "b"], "some body");
        m._arm("Green", vec!["a", "b"], "different body");
        m._arm("Blue", vec!["x", "y"], "some body");
        assert_eq!(m.arms.len(), 3);
    }

    #[test]
    fn parse_multiline_works() {
        let input = "\n    hello\n    world\n";
        let expected = vec!["hello", "world"];
        let output = parse_multiline(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn formatter_basic_example_works() {
        let mut fmt = Formatter::new();
        fmt.line("Hello line 1");
        fmt.indent_push();
        fmt._comment("Nested comment");
        fmt.indent_pop();
        fmt.line("Back home again");
        let expected_lines = vec![
            "Hello line 1\n",
            "    // Nested comment\n",
            "Back home again\n",
        ];
        assert_eq!(fmt.lines, expected_lines);
    }

    #[test]
    fn get_indent_works() {
        let mut fmt = Formatter::new();
        let expected_results = vec!["", "    ", "        ", ""];

        let actual_results = Vec::with_capacity(4);
        (0..3).for_each(|_| {
            fmt.get_indent();
            fmt.indent_push();
        });
        (0..3).for_each(|_| fmt.indent_pop());
        fmt.get_indent();

        actual_results
            .into_iter()
            .zip(expected_results.into_iter())
            .for_each(|(actual, expected): (String, &str)| assert_eq!(&actual, expected));
    }

    #[test]
    fn fmt_can_add_type_to_lines() {
        let mut fmt = Formatter::new();
        fmt.line(&format!("pub const {}: Type = Type({:#x});", "example", 0,));
        let expected_lines = vec!["pub const example: Type = Type(0x0);\n"];
        assert_eq!(fmt.lines, expected_lines);
    }

    #[test]
    fn fmt_can_add_indented_line() {
        let mut fmt = Formatter::new();
        fmt.line("hello");
        fmt.indent_push();
        fmt.line("world");
        let expected_lines = vec!["hello\n", "    world\n"];
        assert_eq!(fmt.lines, expected_lines);
    }

    #[test]
    fn fmt_can_add_doc_comments() {
        let mut fmt = Formatter::new();
        fmt.doc_comment("documentation\nis\ngood");
        let expected_lines = vec!["/// documentation\n", "/// is\n", "/// good\n"];
        assert_eq!(fmt.lines, expected_lines);
    }
}
