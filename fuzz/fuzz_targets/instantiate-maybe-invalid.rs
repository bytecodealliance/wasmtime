#![no_main]

use libfuzzer_sys::fuzz_target;
use std::time::Duration;
use wasm_smith::MaybeInvalidModule;
use wasmtime::Strategy;
use wasmtime_fuzzing::oracles;

fuzz_target!(|module: MaybeInvalidModule| {
    oracles::instantiate_with_config(
        &module.to_bytes(),
        false,
        wasmtime_fuzzing::fuzz_default_config(Strategy::Auto).unwrap(),
        Some(Duration::from_secs(20)),
    );
});
