#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Result, Unstructured};
use wasmtime_fuzzing::generators::Config;
use wasmtime_fuzzing::oracles::{instantiate, Timeout};

wasmtime_fuzzing::single_module_fuzzer!(execute gen_module);

#[derive(Debug)]
struct InstantiateInput {
    config: Config,
    timeout: Timeout,
}

impl<'a> Arbitrary<'a> for InstantiateInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let mut config: Config = u.arbitrary()?;

        // Pick either fuel, duration-based, or module-based timeout. Note that
        // the module-based timeout is implemented with wasm-smith's
        // `ensure_termination` option.
        let timeout = if u.arbitrary()? {
            config.generate_timeout(u)?
        } else {
            Timeout::None
        };

        Ok(InstantiateInput { config, timeout })
    }
}

fn execute(
    module: &[u8],
    known_valid: bool,
    mut input: InstantiateInput,
    u: &mut Unstructured<'_>,
) -> Result<()> {
    let timeout = match input.timeout {
        // If the input module isn't a "known valid" module then it can't be
        // relied on self-regulating itself, so force a timeout via epochs/fuel
        // in the configuration.
        Timeout::None if !known_valid => input.config.generate_timeout(u)?,
        other => other,
    };
    instantiate(module, known_valid, &input.config, timeout);
    Ok(())
}

fn gen_module(input: &mut InstantiateInput, u: &mut Unstructured<'_>) -> Result<Vec<u8>> {
    let module = input.config.generate(
        u,
        if let Timeout::None = input.timeout {
            Some(1000)
        } else {
            None
        },
    )?;
    Ok(module.to_bytes())
}
