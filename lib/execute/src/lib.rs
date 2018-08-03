//! JIT-style runtime for WebAssembly using Cranelift.

#![deny(missing_docs)]

extern crate cranelift_codegen;
extern crate cranelift_wasm;
extern crate region;
extern crate wasmtime_runtime;

mod execute;
mod instance;

pub use execute::{compile_and_link_module, execute};
pub use instance::Instance;
