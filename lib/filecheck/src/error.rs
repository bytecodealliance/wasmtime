use std::result;
use std::convert::From;
use std::error::Error as StdError;
use std::fmt;
use regex;

/// A result from the filecheck library.
pub type Result<T> = result::Result<T, Error>;

/// A filecheck error.
#[derive(Debug)]
pub enum Error {
    /// A syntax error in a check line.
    Syntax(String),
    /// A check refers to an undefined variable.
    ///
    /// The pattern contains `$foo` where the `foo` variable has not yet been defined.
    /// Use `$$` to match a literal dollar sign.
    UndefVariable(String),
    /// A pattern contains a back-reference to a variable that was defined in the same pattern.
    ///
    /// For example, `check: Hello $(world=.*) $world`. Backreferences are not supported. Often the
    /// desired effect can be achieved with the `sameln` check:
    ///
    /// ```text
    /// check: Hello $(world=[^ ]*)
    /// sameln: $world
    /// ```
    Backref(String),
    /// A pattern contains multiple definitions of the same variable.
    DuplicateDef(String),
    /// An error in a regular expression.
    ///
    /// Use `cause()` to get the underlying `Regex` library error.
    Regex(regex::Error),
}

impl StdError for Error {
    fn description(&self) -> &str {
        use Error::*;
        match *self {
            Syntax(ref s) |
            UndefVariable(ref s) |
            Backref(ref s) |
            DuplicateDef(ref s) => s,
            Regex(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        use Error::*;
        match *self {
            Regex(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.description())
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Error {
        Error::Regex(e)
    }
}
