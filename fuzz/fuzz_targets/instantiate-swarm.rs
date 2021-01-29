#![no_main]

use libfuzzer_sys::fuzz_target;
use std::time::Duration;
use wasm_smith::{Config, ConfiguredModule, SwarmConfig};
use wasmtime::Strategy;
use wasmtime_fuzzing::oracles::{self, Timeout};

fuzz_target!(|pair: (bool, ConfiguredModule<SwarmConfig>)| {
    let (timeout_with_time, module) = pair;
    let mut cfg = wasmtime_fuzzing::fuzz_default_config(Strategy::Auto).unwrap();
    cfg.wasm_multi_memory(true);
    cfg.wasm_module_linking(module.config().module_linking_enabled());
    oracles::instantiate_with_config(
        &module.to_bytes(),
        true,
        cfg,
        if timeout_with_time {
            Timeout::Time(Duration::from_secs(20))
        } else {
            Timeout::Fuel(100_000)
        },
    );
});
