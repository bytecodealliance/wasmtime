#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: (
    generators::DifferentialConfig,
    generators::DifferentialConfig,
    generators::WasmOptTtf
)| {
    let (lhs, rhs, wasm) = data;
    oracles::differential_execution(&wasm.wasm, &[lhs, rhs]);
});
