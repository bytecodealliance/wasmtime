#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Result, Unstructured};
use libfuzzer_sys::fuzz_target;

fuzz_target!(
    init: {
        wasmtime_fuzzing::init_fuzzing();
        wasmtime_fuzzing::oracles::component_async::init();
    },
    |bytes: &[u8]| {
        let _ = run(bytes);
    }
);

fn run(bytes: &[u8]) -> Result<()> {
    let u = Unstructured::new(bytes);
    let input = Arbitrary::arbitrary_take_rest(u)?;
    wasmtime_fuzzing::oracles::component_async::run(input);
    Ok(())
}
