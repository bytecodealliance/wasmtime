//! Top-level lib.rs for `cretonne_faerie`.
//!
//! Users of this module should not have to depend on faerie directly.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]

extern crate cretonne_codegen;
extern crate cretonne_module;
extern crate faerie;
#[macro_use]
extern crate failure;
extern crate goblin;

mod backend;
mod container;
mod target;

pub use backend::FaerieBackend;
pub use container::Format;
