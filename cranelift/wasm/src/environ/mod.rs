//! Support for configurable wasm translation.

mod dummy;
#[macro_use]
mod spec;

pub use crate::environ::dummy::DummyEnvironment;
pub use crate::environ::spec::{
    Alias, FuncEnvironment, GlobalVariable, ModuleEnvironment, ReturnMode, TargetEnvironment,
};
