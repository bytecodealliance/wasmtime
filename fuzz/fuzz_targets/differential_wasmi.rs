#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: (
    generators::Config,
    wasm_smith::ConfiguredModule<oracles::SingleFunctionModuleConfig>
)| {
    let (config, mut wasm) = data;
    wasm.module.ensure_termination(1000);
    oracles::differential_wasmi_execution(&wasm.module.to_bytes(), &config);
});
