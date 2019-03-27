#![cfg_attr(feature = "bench", feature(test))]
#![feature(proc_macro_hygiene)]

#[macro_use]
extern crate smallvec;
extern crate capstone;
extern crate either;
extern crate failure;
pub extern crate wasmparser;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate memoffset;
extern crate dynasm;
extern crate dynasmrt;
extern crate itertools;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate wabt;
// Just so we can implement `Signature` for `cranelift_codegen::ir::Signature`
extern crate cranelift_codegen;
extern crate multi_mut;

mod backend;
mod disassemble;
mod error;
mod function_body;
mod microwasm;
mod module;
mod translate_sections;

#[cfg(test)]
mod tests;

pub use crate::backend::CodeGenSession;
pub use crate::function_body::translate_wasm as translate_function;
pub use crate::module::{translate, ExecutableModule, ModuleContext, Signature, TranslatedModule};
