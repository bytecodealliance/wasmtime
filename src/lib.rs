#![feature(plugin, test, const_slice_len)]
#![plugin(dynasm)]

extern crate test;

extern crate arrayvec;
extern crate capstone;
extern crate failure;
extern crate wasmparser;
#[macro_use]
extern crate failure_derive;
extern crate dynasmrt;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate wabt;

mod backend;
mod disassemble;
mod error;
mod function_body;
mod module;
mod translate_sections;

#[cfg(test)]
mod tests;

pub use module::translate;
pub use module::TranslatedModule;
