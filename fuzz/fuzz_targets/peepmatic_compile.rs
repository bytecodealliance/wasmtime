#![no_main]

use libfuzzer_sys::fuzz_target;
use peepmatic_fuzzing::compile::compile;

fuzz_target!(|data: &[u8]| {
    compile(data);
});
