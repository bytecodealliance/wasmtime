#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use std::time::Duration;
use wasm_smith::{Module, SwarmConfig};
use wasmtime::Strategy;
use wasmtime_fuzzing::oracles::{self, Timeout};

fuzz_target!(|data: &[u8]| {
    // errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(data));
});

fn run(data: &[u8]) -> Result<()> {
    let mut u = Unstructured::new(data);
    let timeout = if u.arbitrary()? {
        Timeout::Time(Duration::from_secs(20))
    } else {
        Timeout::Fuel(100_000)
    };

    // Further configure `SwarmConfig` after we generate one to enable features
    // that aren't otherwise enabled by default. We want to test all of these in
    // Wasmtime.
    let mut config: SwarmConfig = u.arbitrary()?;
    config.module_linking_enabled = u.arbitrary()?;
    config.memory64_enabled = u.arbitrary()?;
    // Don't generate modules that allocate more than 6GB
    config.max_memory_pages = 6 << 30;
    let module = Module::new(config.clone(), &mut u)?;

    let mut cfg = wasmtime_fuzzing::fuzz_default_config(Strategy::Auto).unwrap();
    cfg.wasm_multi_memory(config.max_memories > 1);
    cfg.wasm_module_linking(config.module_linking_enabled);
    cfg.wasm_memory64(config.memory64_enabled);

    oracles::instantiate_with_config(&module.to_bytes(), true, cfg, timeout);
    Ok(())
}
