#![no_main]

use libfuzzer_sys::fuzz_target;
use peepmatic_fuzzing::parser::parse;

fuzz_target!(|data: &[u8]| {
    parse(data);
});
