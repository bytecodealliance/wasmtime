#![no_main]

use libfuzzer_sys::fuzz_target;

use cranelift_codegen::{settings, verify_function};
use cranelift_fuzzgen::TestCase;

fuzz_target!(|testcase: TestCase| {
    let flags = settings::Flags::new(settings::builder());
    verify_function(&testcase.func, &flags).unwrap();
});
