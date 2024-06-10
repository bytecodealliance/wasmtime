//! Support for configurable wasm translation.

#[macro_use]
mod spec;

pub use crate::environ::spec::{
    FuncEnvironment, GlobalVariable, ModuleEnvironment, TargetEnvironment,
};
