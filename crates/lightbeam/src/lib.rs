#![cfg_attr(test, feature(test))]
#![feature(proc_macro_hygiene)]

mod alloc;
mod backend;
mod disassemble;
mod error;
mod function_body;
pub mod microwasm;
mod module;
mod translate_sections;

pub use error::Error;
pub use backend::CodeGenSession;
pub use function_body::{
    translate_wasm as translate_function, NullOffsetSink, OffsetSink, Sinks,
};
pub use module::{
    translate, ExecutableModule, ExecutionError, ModuleContext, Signature, TranslatedModule,
};
pub use disassemble::disassemble;
