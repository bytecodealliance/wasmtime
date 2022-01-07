#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::{Engine, Module};
use wasmtime_fuzzing::wasm_smith::MaybeInvalidModule;

fuzz_target!(|module: MaybeInvalidModule| {
    let engine = Engine::default();
    drop(Module::new(&engine, &module.to_bytes()));
});
