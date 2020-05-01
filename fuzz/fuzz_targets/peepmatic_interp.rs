#![no_main]

use libfuzzer_sys::fuzz_target;
use peepmatic_fuzzing::interp::interp;

fuzz_target!(|data: &[u8]| {
    interp(data);
});
