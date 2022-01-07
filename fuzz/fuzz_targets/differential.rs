#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators, oracles};

fuzz_target!(|data: &[u8]| {
    // errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(data));
});

fn run(data: &[u8]) -> Result<()> {
    let mut u = Unstructured::new(data);

    let lhs: generators::WasmtimeConfig = u.arbitrary()?;
    let rhs: generators::WasmtimeConfig = u.arbitrary()?;
    let config: generators::ModuleConfig = u.arbitrary()?;
    let mut module = config.generate(&mut u)?;
    module.ensure_termination(1000);

    oracles::differential_execution(&module.to_bytes(), &config, &[lhs, rhs]);
    Ok(())
}
