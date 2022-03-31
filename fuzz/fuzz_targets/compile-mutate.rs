#![no_main]

use libfuzzer_sys::{fuzz_mutator, fuzz_target};
use wasmtime::{Engine, Module};

fuzz_target!(|data: &[u8]| {
    let engine = Engine::default();
    wasmtime_fuzzing::oracles::log_wasm(data);
    drop(Module::new(&engine, data));
});

fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    wasmtime_fuzzing::mutators::wasm_mutate(
        data,
        size,
        max_size,
        seed,
        libfuzzer_sys::fuzzer_mutate,
    )
});
