#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::{Config, SpecTest};

fuzz_target!(|pair: (Config, SpecTest)| {
    let (config, test) = pair;
    wasmtime_fuzzing::oracles::spectest(config, test);
});
