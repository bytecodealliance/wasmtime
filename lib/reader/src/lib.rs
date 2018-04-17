//! Cretonne file reader library.
//!
//! The `cretonne_reader` library supports reading .cton files. This functionality is needed for
//! testing Cretonne, but is not essential for a JIT compiler.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]

extern crate cretonne;

pub use error::{Error, Location, Result};
pub use isaspec::{parse_options, IsaSpec};
pub use parser::{parse_functions, parse_test};
pub use sourcemap::SourceMap;
pub use testcommand::{TestCommand, TestOption};
pub use testfile::{Comment, Details, TestFile};

mod error;
mod isaspec;
mod lexer;
mod parser;
mod sourcemap;
mod testcommand;
mod testfile;
