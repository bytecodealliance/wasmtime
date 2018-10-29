#![feature(plugin)]
#![plugin(dynasm)]

extern crate capstone;
extern crate failure;
extern crate wasmparser;
#[macro_use]
extern crate failure_derive;
extern crate dynasmrt;

mod backend;
mod disassemble;
mod error;
mod function_body;
mod module;
mod translate_sections;

pub use module::translate;
