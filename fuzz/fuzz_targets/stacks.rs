//! Check that we see the stack trace correctly.

#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators::Stacks, oracles::check_stacks};

fuzz_target!(|stacks: Stacks| {
    check_stacks(stacks);
});
