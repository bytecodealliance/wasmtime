#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::{Engine, Module};

fuzz_target!(|data: &[u8]| {
    let engine = Engine::default();
    drop(Module::new(&engine, data));
});
