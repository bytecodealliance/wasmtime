#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
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

    let module = config.generate(
        &mut u,
        if let Timeout::None = timeout {
            Some(1000)
        } else {
            None
        },
    )?;

    oracles::instantiate(&module.to_bytes(), true, &config, timeout);
    Ok(())
}
