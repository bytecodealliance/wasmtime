#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::InstanceAllocationStrategy;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: &[u8]| {
    // errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(data));
});

fn run(data: &[u8]) -> Result<()> {
    let mut u = Unstructured::new(data);
    let mut config: generators::Config = u.arbitrary()?;
    config.set_differential_config();

    // Enable features that v8 has implemented
    config.module_config.config.simd_enabled = u.arbitrary()?;
    config.module_config.config.bulk_memory_enabled = u.arbitrary()?;
    config.module_config.config.reference_types_enabled = u.arbitrary()?;
    // FIXME: to enable fuzzing with the threads proposal, see
    // https://github.com/bytecodealliance/wasmtime/issues/4268.
    // config.module_config.config.threads_enabled = u.arbitrary()?;

    // Allow multiple tables, as set_differential_config() assumes reference
    // types are disabled and therefore sets max_tables to 1
    config.module_config.config.max_tables = 4;
    if let InstanceAllocationStrategy::Pooling {
        instance_limits: limits,
        ..
    } = &mut config.wasmtime.strategy
    {
        limits.tables = 4;
    }

    let module = config.generate(&mut u, Some(1000))?;
    oracles::differential_v8_execution(&module.to_bytes(), &config);
    Ok(())
}
