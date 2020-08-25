#![no_main]

use libfuzzer_sys::fuzz_target;
use wasm_smith::Module;
use wasmtime::Strategy;
use wasmtime_fuzzing::oracles;

fuzz_target!(|module: Module| {
    let mut module = module;
    module.ensure_termination(1000);
    let wasm_bytes = module.to_bytes();
    oracles::instantiate(&wasm_bytes, Strategy::Auto);
});
