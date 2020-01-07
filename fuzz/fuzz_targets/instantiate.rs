#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::Strategy;
use wasmtime_fuzzing::{oracles, with_log_wasm_test_case};

fuzz_target!(|data: &[u8]| {
    with_log_wasm_test_case!(data, |data| oracles::instantiate(data, Strategy::Auto,));
});
