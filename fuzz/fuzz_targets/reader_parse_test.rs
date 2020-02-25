#![no_main]

use libfuzzer_sys::fuzz_target;

use std::str;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = str::from_utf8(data) {
        let options = cranelift_reader::ParseOptions::default();
        let _ = cranelift_reader::parse_test(s, options);
    }
});
