//! Define the `Location`, `Error`, and `Result` types.

#![macro_use]

use std::fmt;
use std::result;

/// The location of a `Token` or `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Location {
    /// Line number. Command-line arguments are line 0 and source file
    /// lines start from 1.
    pub line_number: usize,
}

/// A parse error is returned when the parse failed.
#[derive(Debug)]
pub struct Error {
    /// Location of the error.
    pub location: Location,
    /// Error message.
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.location.line_number == 0 {
            write!(f, "command-line arguments: {}", self.message)
        } else {
            write!(f, "{}: {}", self.location.line_number, self.message)
        }
    }
}

/// Result of a parser operation. The `Error` variant includes a location.
pub type Result<T> = result::Result<T, Error>;

// Create an `Err` variant of `Result<X>` from a location and `format!` args.
macro_rules! err {
    ( $loc:expr, $msg:expr ) => {
        Err($crate::Error {
            location: $loc.clone(),
            message: $msg.to_string(),
        })
    };

    ( $loc:expr, $fmt:expr, $( $arg:expr ),+ ) => {
        Err($crate::Error {
            location: $loc.clone(),
            message: format!( $fmt, $( $arg ),+ ),
        })
    };
}
