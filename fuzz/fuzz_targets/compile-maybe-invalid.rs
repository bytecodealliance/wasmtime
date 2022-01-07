#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::{Engine, Module};
use wasmtime_fuzzing::wasm_smith::MaybeInvalidModule;

fuzz_target!(|module: MaybeInvalidModule| {
    let engine = Engine::default();
    let wasm = module.to_bytes();
    wasmtime_fuzzing::oracles::log_wasm(&wasm);
    drop(Module::new(&engine, &wasm));
});
