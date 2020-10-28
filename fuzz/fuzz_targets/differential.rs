#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: (
    generators::DifferentialConfig,
    generators::DifferentialConfig,
    wasm_smith::Module,
)| {
    let (lhs, rhs, mut wasm) = data;
    wasm.ensure_termination(1000);
    oracles::differential_execution(&wasm, &[lhs, rhs]);
});
