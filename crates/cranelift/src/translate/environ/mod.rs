//! Support for configurable wasm translation.

#[macro_use]
mod spec;

pub use crate::translate::environ::spec::{
    GlobalConstValue, GlobalVariable, StructFieldsVec, TargetEnvironment,
};
