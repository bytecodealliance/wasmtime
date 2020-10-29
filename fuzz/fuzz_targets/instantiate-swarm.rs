#![no_main]

use libfuzzer_sys::fuzz_target;
use std::time::Duration;
use wasm_smith::{ConfiguredModule, SwarmConfig};
use wasmtime::Strategy;
use wasmtime_fuzzing::oracles;

fuzz_target!(|module: ConfiguredModule<SwarmConfig>| {
    let mut cfg = wasmtime_fuzzing::fuzz_default_config(Strategy::Auto).unwrap();
    cfg.wasm_multi_memory(true);
    oracles::instantiate_with_config(&module.to_bytes(), cfg, Some(Duration::from_secs(20)));
});
