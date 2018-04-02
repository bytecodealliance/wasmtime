//! Performs translation from a wasm module in binary format to the in-memory form
//! of Cretonne IR. More particularly, it translates the code of all the functions bodies and
//! interacts with an environment implementing the
//! [`ModuleEnvironment`](trait.ModuleEnvironment.html)
//! trait to deal with tables, globals and linear memory.
//!
//! The crate provides a `DummyEnvironment` struct that will allow to translate the code of the
//! functions but will fail at execution.
//!
//! The main function of this module is [`translate_module`](fn.translate_module.html).

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces, unstable_features)]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy",
            allow(new_without_default, new_without_default_derive))]
#![cfg_attr(feature="cargo-clippy", warn(
                float_arithmetic,
                mut_mut,
                nonminimal_bool,
                option_map_unwrap_or,
                option_map_unwrap_or_else,
                print_stdout,
                unicode_not_nfc,
                use_self,
                ))]

#[macro_use(dbg)]
extern crate cretonne_codegen;
extern crate cretonne_frontend;
extern crate wasmparser;

mod code_translator;
mod environ;
mod func_translator;
mod module_translator;
mod sections_translator;
mod state;
mod translation_utils;

pub use environ::{DummyEnvironment, FuncEnvironment, GlobalValue, ModuleEnvironment};
pub use func_translator::FuncTranslator;
pub use module_translator::translate_module;
pub use translation_utils::{FunctionIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex,
                            SignatureIndex, Table, TableIndex};
