//! Logic for writing generated code to a file.
//!
//! Use [`fmtln`] by default since it will append Rust comments with the original source location.
//! For more control, use the [`Formatter`] directly. The [`Formatter`] logic has been borrowed
//! extensively from `cranelift/codegen/meta/src/srcgen.rs`.

use std::fs;
use std::io::{self, Write};

static SHIFTWIDTH: usize = 4;

/// A macro that simplifies the usage of the [`Formatter`] by allowing format strings.
macro_rules! fmtln {
    ($fmt:ident, $fmtstring:expr, $($fmtargs:expr),*) => {
        let loc = crate::generate::maybe_file_loc($fmtstring, file!(), line!());
        $fmt.line(format!($fmtstring, $($fmtargs),*), loc)
    };

    ($fmt:ident, $arg:expr) => {{
        let loc = crate::generate::maybe_file_loc($arg, file!(), line!());
        $fmt.line(format!($arg), loc)
    }};

    ($_:tt, $($args:expr),+) => {
        compile_error!("This macro requires at least two arguments: the Formatter instance and a format string.")
    };

    ($_:tt) => {
        compile_error!("This macro requires at least two arguments: the Formatter instance and a format string.")
    };
}
pub(crate) use fmtln;

/// Append a source location comment "intelligently:" only on generated lines that do not start or
/// end a braces block.
pub fn maybe_file_loc(fmtstr: &str, file: &'static str, line: u32) -> Option<FileLocation> {
    if fmtstr.ends_with(['{', '}']) {
        None
    } else {
        Some(FileLocation { file, line })
    }
}

/// Record a source location; preferably, use [`fmtln`] directly.
pub struct FileLocation {
    file: &'static str,
    line: u32,
}

impl FileLocation {
    pub fn new(file: &'static str, line: u32) -> Self {
        Self { file, line }
    }
}

impl core::fmt::Display for FileLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// Collect source code to be written to a file and keep track of indentation.
#[derive(Default)]
pub struct Formatter {
    indent: usize,
    lines: Vec<String>,
}

impl Formatter {
    /// Construct a [`Formatter`].
    ///
    /// This constructor reminds us to add the `generated_by` header (typically
    /// a comment) to the output file.
    pub fn new(generated_by: &str, file: &'static str, line: u32) -> Self {
        let mut fmt = Self::default();
        let loc = FileLocation::new(file, line);
        fmt.line(format!("{generated_by}, {loc}"), None);
        fmt
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

    /// Write all formatting commands in `f` while indented one level.
    pub fn indent<T, F: FnOnce(&mut Formatter) -> T>(&mut self, f: F) -> T {
        self.indent_push();
        let ret = f(self);
        self.indent_pop();
        ret
    }

    /// Get the current whitespace indentation.
    fn get_indent(&self) -> String {
        if self.indent == 0 {
            String::new()
        } else {
            format!("{:-1$}", " ", self.indent * SHIFTWIDTH)
        }
    }

    /// Add an indented line.
    pub fn line(&mut self, contents: impl AsRef<str>, location: Option<FileLocation>) {
        let indented_line = if let Some(location) = location {
            format!("{} {} // {location}\n", self.get_indent(), contents.as_ref())
        } else {
            format!("{}{}\n", self.get_indent(), contents.as_ref())
        };
        self.lines.push(indented_line);
    }

    /// Push an empty line.
    pub fn empty_line(&mut self) {
        self.lines.push("\n".to_string());
    }

    /// Add a comment line.
    pub fn comment(&mut self, s: impl AsRef<str>) {
        self.line(format!("// {}", s.as_ref()), None);
    }

    /// Write the collected lines to a file.
    pub fn write(&self, path: impl AsRef<std::path::Path>) -> io::Result<()> {
        let mut f = fs::File::create(path)?;
        for l in self.lines.iter().map(String::as_bytes) {
            f.write_all(l)?;
        }
        Ok(())
    }
}
