#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};
use wasmtime_jit::CompilationStrategy;

fuzz_target!(|data: generators::WasmOptTtf| {
    oracles::instantiate(&data.wasm, CompilationStrategy::Auto);
});
