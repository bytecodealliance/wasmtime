#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles, with_log_wasm_test_case};
use wasmtime_jit::CompilationStrategy;

fuzz_target!(|data: generators::WasmOptTtf| {
    with_log_wasm_test_case!(&data.wasm, |wasm| oracles::instantiate(
        wasm,
        CompilationStrategy::Auto
    ));
});
