//! Performs translation from a wasm module in binary format to the in-memory form
//! of Cranelift IR. More particularly, it translates the code of all the functions bodies and
//! interacts with an environment implementing the
//! [`ModuleEnvironment`](trait.ModuleEnvironment.html)
//! trait to deal with tables, globals and linear memory.
//!
//! The crate provides a `DummyEnvironment` struct that will allow to translate the code of the
//! functions but will fail at execution.
//!
//! The main function of this module is [`translate_module`](fn.translate_module.html).

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![cfg_attr(feature = "std", deny(unstable_features))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy::float_arithmetic,
        clippy::mut_mut,
        clippy::nonminimal_bool,
        clippy::option_map_unwrap_or,
        clippy::option_map_unwrap_or_else,
        clippy::print_stdout,
        clippy::unicode_not_nfc,
        clippy::use_self
    )
)]
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
mod module_translator;
mod sections_translator;
mod state;
mod translation_utils;

pub use crate::environ::{
    DummyEnvironment, FuncEnvironment, GlobalVariable, ModuleEnvironment, ReturnMode,
    TargetEnvironment, WasmError, WasmResult,
};
pub use crate::func_translator::FuncTranslator;
pub use crate::module_translator::translate_module;
pub use crate::state::func_state::FuncTranslationState;
pub use crate::state::module_state::ModuleTranslationState;
pub use crate::translation_utils::{
    get_vmctx_value_label, DefinedFuncIndex, DefinedGlobalIndex, DefinedMemoryIndex,
    DefinedTableIndex, FuncIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex,
    PassiveDataIndex, PassiveElemIndex, SignatureIndex, Table, TableElementType, TableIndex,
};

/// Version number of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
