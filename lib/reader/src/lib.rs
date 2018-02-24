//! Cretonne file reader library.
//!
//! The cton_reader library supports reading .cton files. This functionality is needed for testing
//! Cretonne, but is not essential for a JIT compiler.

#![deny(missing_docs,
        trivial_numeric_casts,
        unused_extern_crates)]

extern crate cretonne;

pub use error::{Location, Result, Error};
pub use parser::{parse_functions, parse_test};
pub use testcommand::{TestCommand, TestOption};
pub use testfile::{TestFile, Details, Comment};
pub use isaspec::{IsaSpec, parse_options};
pub use sourcemap::SourceMap;

mod error;
mod lexer;
mod parser;
mod testcommand;
mod isaspec;
mod testfile;
mod sourcemap;
