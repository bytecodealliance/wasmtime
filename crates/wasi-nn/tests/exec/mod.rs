//! Provide a Wasmtime embedding for executing wasi-nn test programs.

pub mod wit;
pub mod witx;

pub const PREOPENED_DIR_NAME: &str = "fixture";
