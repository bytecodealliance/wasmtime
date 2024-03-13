#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::{Config, WastTest};

fuzz_target!(|pair: (Config, WastTest)| {
    let (config, test) = pair;
    wasmtime_fuzzing::oracles::wast_test(config, test);
});
