#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::{generators::MemoryAccesses, oracles::memory::check_memory_accesses};

fuzz_target!(|input: MemoryAccesses| {
    check_memory_accesses(input);
});
