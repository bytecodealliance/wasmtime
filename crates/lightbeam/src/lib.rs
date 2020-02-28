#![cfg_attr(test, feature(test))]
#![feature(proc_macro_hygiene)]

#[cfg(test)]
extern crate test;

mod backend;
mod disassemble;
mod error;
mod function_body;
mod microwasm;
mod module;
mod translate_sections;

#[cfg(test)]
mod benches;

pub use crate::backend::CodeGenSession;
pub use crate::function_body::translate_wasm as translate_function;
pub use crate::module::{
    translate, ExecutableModule, ExecutionError, ModuleContext, Signature, TranslatedModule,
};
