//! Support for configurable wasm translation.

mod spec;
mod dummy;

pub use environ::spec::{FuncEnvironment, GlobalValue, ModuleEnvironment};
pub use environ::dummy::DummyEnvironment;
