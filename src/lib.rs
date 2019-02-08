#![feature(
    plugin,
    test,
    const_slice_len,
    never_type,
    alloc_layout_extra,
    try_from
)]
#![plugin(dynasm)]

extern crate test;

extern crate arrayvec;
extern crate capstone;
extern crate failure;
pub extern crate wasmparser;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate memoffset;
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

mod backend;
mod disassemble;
mod error;
mod function_body;
mod module;
mod translate_sections;

#[cfg(test)]
mod tests;

pub use backend::CodeGenSession;
pub use function_body::translate as translate_function;
pub use module::{translate, ExecutableModule, ModuleContext, Signature, TranslatedModule};
