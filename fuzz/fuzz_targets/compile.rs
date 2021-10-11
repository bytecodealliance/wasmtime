#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime::Strategy;
use wasmtime_fuzzing::oracles;

fuzz_target!(|data: &[u8]| {
    oracles::compile(data, Strategy::Cranelift);
});
