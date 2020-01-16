#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::Strategy;
use wasmtime_fuzzing::{oracles, with_log_wasm_test_case};

fuzz_target!(|data: &[u8]| {
    with_log_wasm_test_case!(data, |data| oracles::compile(data, Strategy::Cranelift));
});

#[cfg(feature = "lightbeam")]
fuzz_target!(|data: &[u8]| {
    with_log_wasm_test_case!(data, |data| oracles::compile(data, Strategy::Lightbeam));
});
