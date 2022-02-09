#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::InstanceAllocationStrategy;
use wasmtime_fuzzing::oracles::Timeout;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: &[u8]| {
    // errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(data));
});

fn run(data: &[u8]) -> Result<()> {
    let mut u = Unstructured::new(data);
    let mut config: generators::Config = u.arbitrary()?;

    // Pick either fuel, duration-based, or module-based timeout. Note that the
    // module-based timeout is implemented with wasm-smith's
    // `ensure_termination` option.
    let timeout = if u.arbitrary()? {
        config.generate_timeout(&mut u)?
    } else {
        Timeout::None
    };

    // Enable module linking for this fuzz target specifically
    config.module_config.config.module_linking_enabled = u.arbitrary()?;

    // When using the pooling allocator without a timeout, we must
    // allow at least 1 more global because the `ensure_termination` call below
    // will define one.
    if let Timeout::None = timeout {
        if let InstanceAllocationStrategy::Pooling { module_limits, .. } =
            &mut config.wasmtime.strategy
        {
            module_limits.globals += 1;
        }
    }

    let mut module = config.module_config.generate(&mut u)?;
    if let Timeout::None = timeout {
        module.ensure_termination(1000);
    }
    oracles::instantiate(&module.to_bytes(), true, &config, timeout);
    Ok(())
}
