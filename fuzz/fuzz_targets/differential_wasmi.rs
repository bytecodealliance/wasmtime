#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: (
    generators::Config,
    wasm_smith::ConfiguredModule<oracles::DifferentialWasmiModuleConfig>
)| {
    let (config, mut wasm) = data;
    wasm.ensure_termination(1000);
    oracles::differential_wasmi_execution(&wasm.to_bytes()[..], &config);
});
