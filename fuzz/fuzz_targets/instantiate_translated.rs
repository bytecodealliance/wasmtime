#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::Strategy;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: generators::WasmOptTtf| {
    oracles::instantiate(&data.wasm, Strategy::Auto);
});
