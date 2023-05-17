//! Compile arbitrary bytes from the fuzzer as if they were Wasm. Also use
//! `wasm-mutate` to mutate the fuzz inputs.

#![no_main]

use libfuzzer_sys::{fuzz_mutator, fuzz_target};
use wasmtime::{Config, Engine, Module};

fn create_engine() -> Engine {
    let mut config = Config::default();
    // Safety: the Cranelift option `regalloc_checker` does not alter
    // the generated code at all; it only does extra checking after
    // compilation.
    unsafe {
        config.cranelift_flag_enable("regalloc_checker");
    }
    Engine::new(&config).expect("Could not construct Engine")
}

fuzz_target!(|data: &[u8]| {
    let engine = create_engine();
    wasmtime_fuzzing::oracles::log_wasm(data);
    drop(Module::new(&engine, data));
});

fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    // Half of the time use libfuzzer's built in mutators, and the other half of
    // the time use `wasm-mutate`.
    if seed.count_ones() % 2 == 0 {
        libfuzzer_sys::fuzzer_mutate(data, size, max_size)
    } else {
        wasmtime_fuzzing::mutators::wasm_mutate(
            data,
            size,
            max_size,
            seed,
            libfuzzer_sys::fuzzer_mutate,
        )
    }
});
