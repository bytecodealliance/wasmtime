//! Define the `Location`, `ParseError`, and `ParseResult` types.

#![macro_use]

use std::fmt;

/// The location of a `Token` or `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Location {
    /// Line number. Command-line arguments are line 0 and source file
    /// lines start from 1.
    pub line_number: usize,
}

/// A parse error is returned when the parse failed.
#[derive(Debug)]
pub struct ParseError {
    /// Location of the error.
    pub location: Location,
    /// Error message.
    pub message: String,
    /// Whether it's a warning or a plain error.
    pub is_warning: bool,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.location.line_number == 0 {
            write!(f, "command-line arguments: {}", self.message)
        } else {
            write!(f, "{}: {}", self.location.line_number, self.message)
        }
    }
}

impl std::error::Error for ParseError {}

/// Result of a parser operation. The `ParseError` variant includes a location.
pub type ParseResult<T> = Result<T, ParseError>;

// Create an `Err` variant of `ParseResult<X>` from a location and `format!` args.
macro_rules! err {
    ( $loc:expr, $msg:expr ) => {
        Err($crate::ParseError {
            location: $loc.clone(),
            message: $msg.to_string(),
            is_warning: false,
        })
    };

    ( $loc:expr, $fmt:expr, $( $arg:expr ),+ ) => {
        Err($crate::ParseError {
            location: $loc.clone(),
            message: format!( $fmt, $( $arg ),+ ),
            is_warning: false,
        })
    };
}

macro_rules! warn {
    ( $loc:expr, $fmt:expr, $( $arg:expr ),+ ) => {
        Err($crate::ParseError {
            location: $loc.clone(),
            message: format!($fmt, $( $arg ),+ ),
            is_warning: true,
        })
    };
}
