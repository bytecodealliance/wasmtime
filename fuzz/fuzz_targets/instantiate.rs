#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::Config;
use wasmtime_fuzzing::oracles::{instantiate, Timeout};
use wasmtime_fuzzing::wasm_smith::Module;

#[derive(Debug)]
struct InstantiateInput {
    config: Config,
    timeout: Timeout,
    module: Module,
}

impl<'a> Arbitrary<'a> for InstantiateInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let mut config: Config = u.arbitrary()?;

        // Pick either fuel, duration-based, or module-based timeout. Note that the
        // module-based timeout is implemented with wasm-smith's
        // `ensure_termination` option.
        let timeout = if u.arbitrary()? {
            config.generate_timeout(u)?
        } else {
            Timeout::None
        };

        let module = config.generate(
            u,
            if let Timeout::None = timeout {
                Some(1000)
            } else {
                None
            },
        )?;

        Ok(InstantiateInput {
            config,
            timeout,
            module,
        })
    }
}

fuzz_target!(|data: InstantiateInput| {
    instantiate(&data.module.to_bytes(), true, &data.config, data.timeout);
});
