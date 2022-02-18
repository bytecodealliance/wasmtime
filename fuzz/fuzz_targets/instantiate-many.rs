//! This fuzz target is used to test multiple concurrent instantiations from
//! multiple modules.

#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};

const MAX_MODULES: usize = 5;

fuzz_target!(|data: &[u8]| {
    // errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(data));
});

fn run(data: &[u8]) -> Result<()> {
    let mut u = Unstructured::new(data);
    let mut config: generators::Config = u.arbitrary()?;

    // Don't generate start functions
    // No wasm code execution is necessary for this fuzz target and thus we don't
    // use timeouts or ensure that the generated wasm code will terminate.
    config.module_config.config.allow_start_export = false;

    // Create the modules to instantiate
    let modules = (0..u.int_in_range(1..=MAX_MODULES)?)
        .map(|_| Ok(config.generate(&mut u, None)?.to_bytes()))
        .collect::<Result<Vec<_>>>()?;

    let max_instances = match &config.wasmtime.strategy {
        generators::InstanceAllocationStrategy::OnDemand => u.int_in_range(1..=100)?,
        generators::InstanceAllocationStrategy::Pooling {
            instance_limits, ..
        } => instance_limits.count,
    };

    // Front-load with instantiation commands
    let mut commands: Vec<oracles::Command> = (0..u.int_in_range(1..=max_instances)?)
        .map(|_| Ok(oracles::Command::Instantiate(u.arbitrary()?)))
        .collect::<Result<_>>()?;

    // Then add some more arbitrary commands
    commands.extend(
        (0..u.int_in_range(0..=2 * max_instances)?)
            .map(|_| u.arbitrary())
            .collect::<Result<Vec<_>>>()?,
    );

    oracles::instantiate_many(&modules, true, &config, &commands);

    Ok(())
}
