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
#![cfg_attr(feature = "cargo-clippy", allow(new_without_default, new_without_default_derive))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        float_arithmetic, mut_mut, nonminimal_bool, option_map_unwrap_or, option_map_unwrap_or_else,
        print_stdout, unicode_not_nfc, use_self
    )
)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

extern crate cranelift_codegen;
#[macro_use]
extern crate cranelift_entity;
extern crate cranelift_frontend;
extern crate target_lexicon;
extern crate wasmparser;

extern crate failure;
#[macro_use]
extern crate failure_derive;

#[macro_use]
extern crate log;

mod code_translator;
mod environ;
mod func_translator;
mod module_translator;
mod sections_translator;
mod state;
mod translation_utils;

pub use environ::{
    DummyEnvironment, FuncEnvironment, GlobalVariable, ModuleEnvironment, WasmError, WasmResult,
};
pub use func_translator::FuncTranslator;
pub use module_translator::translate_module;
pub use translation_utils::{
    DefinedFuncIndex, FuncIndex, Global, GlobalIndex, GlobalInit, Memory, MemoryIndex,
    SignatureIndex, Table, TableIndex,
};

#[cfg(not(feature = "std"))]
mod std {
    extern crate alloc;

    pub use self::alloc::string;
    pub use self::alloc::vec;
    pub use core::fmt;
    pub use core::option;
    pub use core::{cmp, i32, str, u32};
    pub mod collections {
        #[allow(unused_extern_crates)]
        extern crate hashmap_core;

        pub use self::hashmap_core::{map as hash_map, HashMap};
    }
}
