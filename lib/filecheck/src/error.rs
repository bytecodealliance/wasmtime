use std::result;
use std::convert::From;
use regex;

/// A result from the filecheck library.
pub type Result<T> = result::Result<T, Error>;

/// A filecheck error.
#[derive(Fail, Debug)]
pub enum Error {
    /// A syntax error in a check line.
    #[fail(display = "{}", _0)]
    Syntax(String),
    /// A check refers to an undefined variable.
    ///
    /// The pattern contains `$foo` where the `foo` variable has not yet been defined.
    /// Use `$$` to match a literal dollar sign.
    #[fail(display = "{}", _0)]
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
    #[fail(display = "{}", _0)]
    Backref(String),
    /// A pattern contains multiple definitions of the same variable.
    #[fail(display = "{}", _0)]
    DuplicateDef(String),
    /// An error in a regular expression.
    ///
    /// Use `cause()` to get the underlying `Regex` library error.
    #[fail(display = "{}", _0)]
    Regex(
        #[cause]
        regex::Error
    ),
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Error {
        Error::Regex(e)
    }
}
