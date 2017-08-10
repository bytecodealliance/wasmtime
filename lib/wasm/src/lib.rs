//! Performs the translation from a wasm module in binary format to the in-memory representation
//! of the Cretonne IL. More particularly, it translates the code of all the functions bodies and
//! interacts with a runtime implementing the [`WasmRuntime`](trait.WasmRuntime.html) trait to
//! deal with tables, globals and linear memory.
//!
//! The crate provides a `DummyRuntime` trait that will allow to translate the code of the
//! functions but will fail at execution. You should use
//! [`wasmstandalone::StandaloneRuntime`](../wasmstandalone/struct.StandaloneRuntime.html) to be
//! able to execute the translated code.
//!
//! The main function of this module is [`translate_module`](fn.translate_module.html).

extern crate wasmparser;
extern crate cton_frontend;
extern crate cretonne;

mod module_translator;
mod translation_utils;
mod code_translator;
mod runtime;
mod sections_translator;

pub use module_translator::{translate_module, TranslationResult, FunctionTranslation,
                            ImportMappings};
pub use runtime::{WasmRuntime, DummyRuntime};
pub use translation_utils::{Local, FunctionIndex, GlobalIndex, TableIndex, MemoryIndex, RawByte,
                            MemoryAddress, SignatureIndex, Global, GlobalInit, Table, Memory};
