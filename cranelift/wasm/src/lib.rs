//! Performs translation from a wasm module in binary format to the in-memory form
//! of Cranelift IR. More particularly, it translates the code of all the functions bodies and
//! interacts with an environment implementing the
//! [`ModuleEnvironment`](trait.ModuleEnvironment.html)
//! trait to deal with tables, globals and linear memory.
//!
//! The main function of this module is [`translate_module`](fn.translate_module.html).

#![deny(missing_docs)]
#![no_std]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc as std;
#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
use hashbrown::{
    hash_map,
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
#[cfg(feature = "std")]
use std::collections::{
    hash_map,
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};

mod code_translator;
mod environ;
mod func_translator;
mod heap;
mod module_translator;
mod sections_translator;
mod state;
mod table;
mod translation_utils;

pub use crate::environ::{FuncEnvironment, GlobalVariable, ModuleEnvironment, TargetEnvironment};
pub use crate::func_translator::FuncTranslator;
pub use crate::heap::{Heap, HeapData, HeapStyle};
pub use crate::module_translator::translate_module;
pub use crate::state::FuncTranslationState;
pub use crate::table::{TableData, TableSize};
pub use crate::translation_utils::*;
pub use cranelift_frontend::FunctionBuilder;
pub use wasmtime_types::*;

// Convenience reexport of the wasmparser crate that we're linking against,
// since a number of types in `wasmparser` show up in the public API of
// `cranelift-wasm`.
pub use wasmparser;

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
