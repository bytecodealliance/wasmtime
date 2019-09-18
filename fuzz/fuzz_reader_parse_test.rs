#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate cranelift_reader;
use std::str;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = str::from_utf8(data) {
        let options = cranelift_reader::ParseOptions::default();
        let _ = cranelift_reader::parse_test(s, options);
    }
});
