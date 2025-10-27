use regex::Regex;
use std::sync::LazyLock;

/// A regex that matches numbers that start with "1".
static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^1\d*$").unwrap());

#[unsafe(export_name = "wizer-initialize")]
pub fn init() {
    LazyLock::force(&REGEX);
}

#[unsafe(no_mangle)]
pub fn run(n: i32) -> i32 {
    let s = format!("{n}");
    if REGEX.is_match(&s) { 42 } else { 0 }
}
