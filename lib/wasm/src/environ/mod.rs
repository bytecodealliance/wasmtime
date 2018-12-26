//! Support for configurable wasm translation.

mod dummy;
mod spec;

pub use crate::environ::dummy::DummyEnvironment;
pub use crate::environ::spec::{
    FuncEnvironment, GlobalVariable, ModuleEnvironment, ReturnMode, WasmError, WasmResult,
};
