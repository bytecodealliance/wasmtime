#![no_main]
use libfuzzer_sys::fuzz_target;
use peepmatic_fuzzing::automata::simple_automata;

fuzz_target!(|input_output_pairs: Vec<Vec<(u8, Vec<u8>)>>| {
    simple_automata(input_output_pairs);
});
