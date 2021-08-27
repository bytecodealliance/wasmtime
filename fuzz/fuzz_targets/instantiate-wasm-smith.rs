#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::Strategy;
use wasmtime_fuzzing::{generators::GeneratedModule, oracles};

fuzz_target!(|module: GeneratedModule| {
    let mut module = module;
    module.module.ensure_termination(1000);
    let wasm_bytes = module.module.to_bytes();
    oracles::instantiate(&wasm_bytes, true, Strategy::Auto);
});
