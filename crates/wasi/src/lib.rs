extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_wasm;
extern crate target_lexicon;
extern crate wasmtime_environ;
extern crate wasmtime_jit;
extern crate wasmtime_runtime;
#[macro_use]
extern crate log;
extern crate wasi_common;

mod instantiate;
mod syscalls;

pub use instantiate::instantiate_wasi;
