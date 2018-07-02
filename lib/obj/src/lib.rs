extern crate cretonne_codegen;
extern crate cretonne_wasm;
extern crate faerie;
extern crate wasmtime_runtime;

mod emit_module;

pub use emit_module::emit_module;
