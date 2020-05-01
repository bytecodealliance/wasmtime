#![no_main]
use libfuzzer_sys::fuzz_target;
use peepmatic_fuzzing::automata::fst_differential;
use std::collections::HashMap;

fuzz_target!(|map: HashMap<Vec<u8>, u64>| {
    fst_differential(map);
});
