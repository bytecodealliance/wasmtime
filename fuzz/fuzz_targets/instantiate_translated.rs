#![no_main]

extern crate libfuzzer_sys;

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};
use wasmtime_jit::CompilationStrategy;

fuzz_target!(|data: generators::WasmOptTtf| {
    oracles::instantiate(&data.wasm, CompilationStrategy::Auto);
});
