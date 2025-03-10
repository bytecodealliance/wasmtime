#![no_main]

use libfuzzer_sys::arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Errors in `wast_test` have to do with not enough input in `data` or the
    // test case being thrown out, which we ignore here since it doesn't affect
    // how we'd like to fuzz.
    let mut u = Unstructured::new(data);
    let _ = wasmtime_fuzzing::oracles::wast_test(&mut u);
});
