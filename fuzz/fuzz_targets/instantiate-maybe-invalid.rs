#![no_main]

use libfuzzer_sys::fuzz_target;
use std::time::Duration;
use wasm_smith::MaybeInvalidModule;
use wasmtime::Strategy;
use wasmtime_fuzzing::oracles::{self, Timeout};

fuzz_target!(|pair: (bool, MaybeInvalidModule)| {
    let (timeout_with_time, module) = pair;
    oracles::instantiate_with_config(
        &module.to_bytes(),
        false,
        wasmtime_fuzzing::fuzz_default_config(Strategy::Auto).unwrap(),
        if timeout_with_time {
            Timeout::Time(Duration::from_secs(20))
        } else {
            Timeout::Fuel(100_000)
        },
    );
});
