//! Performs the translation from a wasm module in binary format to the in-memory representation
//! of the Cretonne IL. More particularly, it translates the code of all the functions bodies and
//! interacts with an environment implementing the
//! [`ModuleEnvironment`](trait.ModuleEnvironment.html)
//! trait to deal with tables, globals and linear memory.
//!
//! The crate provides a `DummyEnvironment` struct that will allow to translate the code of the
//! functions but will fail at execution.
//!
//! The main function of this module is [`translate_module`](fn.translate_module.html).

#![deny(missing_docs)]

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

#[cfg(feature = "no_std")]
extern crate hashmap_core;

extern crate wasmparser;
extern crate cton_frontend;
#[macro_use(dbg)]
extern crate cretonne;

mod code_translator;
mod func_translator;
mod module_translator;
mod environ;
mod sections_translator;
mod state;
mod translation_utils;

pub use func_translator::FuncTranslator;
pub use module_translator::translate_module;
pub use environ::{FuncEnvironment, ModuleEnvironment, DummyEnvironment, GlobalValue};
pub use translation_utils::{FunctionIndex, GlobalIndex, TableIndex, MemoryIndex, SignatureIndex,
                            Global, GlobalInit, Table, Memory};

#[cfg(not(feature = "std"))]
mod std {
    pub use alloc::vec;
    pub use alloc::string;
    pub use core::{u32, i32, str, cmp};
    pub mod collections {
        pub use hashmap_core::{HashMap, map as hash_map};
    }
}
