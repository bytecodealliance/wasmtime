//! Top-level lib.rs for `cretonne_simplejit`.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]

extern crate cretonne_codegen;
extern crate cretonne_module;
extern crate cretonne_native;
extern crate errno;
extern crate region;
extern crate libc;

mod backend;
mod memory;

pub use backend::SimpleJITBackend;
