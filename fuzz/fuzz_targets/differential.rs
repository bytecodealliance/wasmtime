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
    let module = config.generate(&mut u, Some(1000))?;

    let lhs = config.wasmtime;
    let mut rhs: generators::WasmtimeConfig = u.arbitrary()?;

    // Use the same allocation strategy between the two configs.
    //
    // Ideally this wouldn't be necessary, but if the lhs is using ondemand
    // and the rhs is using the pooling allocator (or vice versa), then
    // the module may have been generated in such a way that is incompatible
    // with the other allocation strategy.
    //
    // We can remove this in the future when it's possible to access the
    // fields of `wasm_smith::Module` to constrain the pooling allocator
    // based on what was actually generated.
    rhs.strategy = lhs.strategy.clone();
    if let InstanceAllocationStrategy::Pooling { .. } = &rhs.strategy {
        // Also use the same memory configuration when using the pooling allocator
        rhs.memory_config = lhs.memory_config.clone();
    }

    oracles::differential_execution(&module.to_bytes(), &config.module_config, &[lhs, rhs]);
    Ok(())
}
