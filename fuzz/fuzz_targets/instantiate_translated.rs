#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::Strategy;
use wasmtime_fuzzing::{generators, oracles, with_log_wasm_test_case};

fuzz_target!(|data: generators::WasmOptTtf| {
    with_log_wasm_test_case!(&data.wasm, |wasm| oracles::instantiate(
        wasm,
        Strategy::Auto,
    ));
});
