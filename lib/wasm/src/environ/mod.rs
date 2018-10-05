//! Support for configurable wasm translation.

mod dummy;
mod spec;

pub use environ::dummy::DummyEnvironment;
pub use environ::spec::{
    FuncEnvironment, GlobalVariable, ModuleEnvironment, ReturnMode, WasmError, WasmResult,
};
