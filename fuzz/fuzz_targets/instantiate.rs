#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{oracles, with_log_wasm_test_case};
use wasmtime_jit::CompilationStrategy;

fuzz_target!(|data: &[u8]| {
    with_log_wasm_test_case!(data, |data| oracles::instantiate(
        data,
        CompilationStrategy::Auto
    ));
});
