//! Cretonne file reader library.
//!
//! The `cretonne_reader` library supports reading .cton files. This functionality is needed for
//! testing Cretonne, but is not essential for a JIT compiler.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces, unstable_features)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default, new_without_default_derive))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        print_stdout, unicode_not_nfc, use_self
    )
)]

extern crate cretonne_codegen;

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
