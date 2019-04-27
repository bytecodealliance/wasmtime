extern crate cast;
extern crate cranelift_codegen;
extern crate cranelift_entity;
extern crate cranelift_wasm;
extern crate target_lexicon;
extern crate wasmtime_environ;
extern crate wasmtime_jit;
extern crate wasmtime_runtime;
#[macro_use]
extern crate log;

mod host;
mod host_impls;
mod instantiate;
mod syscalls;
mod translate;
mod wasm32;

pub use instantiate::instantiate_wasi;
