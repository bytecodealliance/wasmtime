#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::oracles;
use wasmtime_jit::{CompilationStrategy};

fuzz_target!(|data: &[u8]| {
    oracles::instantiate(data, CompilationStrategy::Auto);
});
