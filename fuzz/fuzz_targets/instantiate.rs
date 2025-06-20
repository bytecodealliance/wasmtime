#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Result, Unstructured};
use wasmtime_fuzzing::generators::Config;
use wasmtime_fuzzing::oracles::{Timeout, instantiate};
use wasmtime_fuzzing::single_module_fuzzer::KnownValid;

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
    known_valid: KnownValid,
    mut input: InstantiateInput,
    u: &mut Unstructured<'_>,
) -> Result<()> {
    let timeout = match input.timeout {
        // If the input module isn't a "known valid" module then it can't be
        // relied on self-regulating itself, so force a timeout via epochs/fuel
        // in the configuration.
        Timeout::None if known_valid == KnownValid::No => input.config.generate_timeout(u)?,
        other => other,
    };
    instantiate(module, known_valid, &input.config, timeout);
    Ok(())
}

fn gen_module(
    input: &mut InstantiateInput,
    u: &mut Unstructured<'_>,
) -> Result<(Vec<u8>, KnownValid)> {
    // With a small-ish chance take raw fuzz input and put it in the module to
    // stress module compilation/validation. In such a situation we can't use
    // `ensure_termination` in wasm-smith so list the timeout as `None` to time
    // out via epochs or Wasmtime-level fuel.
    //
    // Otherwise though if no timeout is configured use wasm-smith fuel to
    // ensure termination.
    let allow_invalid_funcs = u.ratio(1, 10)?;

    let default_fuel = if allow_invalid_funcs {
        input.config.module_config.config.allow_invalid_funcs = true;
        input.timeout = Timeout::None;
        None
    } else if let Timeout::None = input.timeout {
        Some(1000)
    } else {
        None
    };
    let module = input.config.generate(u, default_fuel)?;
    let known_valid = if allow_invalid_funcs {
        KnownValid::No
    } else {
        KnownValid::Yes
    };
    Ok((module.to_bytes(), known_valid))
}
