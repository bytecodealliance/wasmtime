//! Compile arbitrary bytes from the fuzzer as if they were Wasm, checking that
//! compilation is deterministic.
//!
//! Also use `wasm-mutate` to mutate the fuzz inputs.

#![no_main]

use libfuzzer_sys::{fuzz_mutator, fuzz_target};
use wasmtime::{Config, Engine, Module, Result};

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

fn compile_and_serialize(engine: &Engine, wasm: &[u8]) -> Result<Vec<u8>> {
    let module = Module::new(&engine, wasm)?;
    module.serialize()
}

fuzz_target!(|data: &[u8]| {
    let engine = create_engine();
    wasmtime_fuzzing::oracles::log_wasm(data);

    if let Ok(bytes1) = compile_and_serialize(&engine, data) {
        let bytes2 = compile_and_serialize(&engine, data)
            .expect("successfully compiled once, should successfully compile again");

        // NB: Don't use `assert_eq!` here because it prints out the LHS and RHS
        // to stderr on failure, which isn't helpful here since it is just a
        // huge serialized binary.
        assert!(bytes1 == bytes2, "Wasm compilation should be deterministic");
    }
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
