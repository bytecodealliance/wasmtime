//! Cretonne file reader library.
//!
//! The cton_reader library supports reading .cton files. This functionality is needed for testing
//! Cretonne, but is not essential for a JIT compiler.

extern crate cretonne;

pub use testcommand::{TestCommand, TestOption};

pub mod lexer;
pub mod parser;
mod testcommand;
