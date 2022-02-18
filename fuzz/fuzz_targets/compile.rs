#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::{Engine, Module};

fuzz_target!(|data: &[u8]| {
    let engine = Engine::default();
    wasmtime_fuzzing::oracles::log_wasm(data);
    drop(Module::new(&engine, data));
});
