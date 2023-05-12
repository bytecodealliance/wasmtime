//! Support for configurable wasm translation.

mod dummy;
#[macro_use]
mod spec;

pub use crate::environ::dummy::{
    DummyEnvironment, DummyFuncEnvironment, DummyModuleInfo, ExpectedReachability,
};
pub use crate::environ::spec::{
    FuncEnvironment, GlobalVariable, ModuleEnvironment, TargetEnvironment,
};
