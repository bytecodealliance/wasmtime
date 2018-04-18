//! Top-level lib.rs for `cretonne_module`.

#![deny(missing_docs, trivial_numeric_casts, unused_extern_crates)]

extern crate cretonne_codegen;
#[macro_use]
extern crate cretonne_entity;

mod backend;
mod data_context;
mod module;

pub use backend::Backend;
pub use data_context::{DataContext, Writability, DataDescription, Init};
pub use module::{DataId, FuncId, Linkage, Module, ModuleNamespace};
