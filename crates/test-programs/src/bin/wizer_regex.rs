use regex::Regex;
use std::sync::LazyLock;

/// A regex that matches numbers that start with "1".
static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^1\d*$").unwrap());

// Component exports will have a name conflict with `wizer-initialize` and
// `run`, so change the core module exports to have prefix `module-`.
#[unsafe(export_name = "module-wizer-initialize")]
pub fn init() {
    LazyLock::force(&REGEX);
}

#[unsafe(export_name = "module-run")]
pub fn run(n: i32) -> i32 {
    let s = format!("{n}");
    if REGEX.is_match(&s) { 42 } else { 0 }
}

/// Stub so that Cargo can build this test as a binary
pub fn main() {
    eprintln!("dont use as a command");
    std::process::exit(-1)
}

wit_bindgen::generate!({
    path: "../wizer/tests/all",
    world: "wizer-test",
});

pub struct C;
export!(C);

impl Guest for C {
    fn wizer_initialize() {
        init()
    }
    fn run(arg: i32) -> i32 {
        run(arg)
    }
}
