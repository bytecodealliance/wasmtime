use regex::Regex;
use std::sync::LazyLock;

/// A regex that matches numbers that start with "1".
static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^1\d*$").unwrap());

#[unsafe(export_name = "wizer-initialize")]
pub extern "C" fn init() {
    LazyLock::force(&REGEX);
}

#[unsafe(export_name = "run")]
pub extern "C" fn run(ptr: *mut u8, len: usize) -> i32 {
    let s = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8(slice).unwrap()
    };
    REGEX.is_match(&s) as u8 as i32
}
