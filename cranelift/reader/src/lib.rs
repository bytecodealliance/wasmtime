//! Cranelift file reader library.
//!
//! The `cranelift_reader` library supports reading .clif files. This functionality is needed for
//! testing Cranelift, but is not essential for a JIT compiler.

#![deny(
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unstable_features
)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]

pub use crate::error::{Location, ParseError, ParseResult};
pub use crate::isaspec::{parse_options, IsaSpec};
pub use crate::parser::{parse_functions, parse_test, ParseOptions};
pub use crate::sourcemap::SourceMap;
pub use crate::testcommand::{TestCommand, TestOption};
pub use crate::testfile::{Comment, Details, Feature, TestFile};

mod error;
mod isaspec;
mod lexer;
mod parser;
mod sourcemap;
mod testcommand;
mod testfile;
