use regex::Regex;

/// A regex that matches numbers that start with "1".
static mut REGEX: Option<Regex> = None;

#[export_name = "wizer.initialize"]
pub fn init() {
    unsafe {
        REGEX = Some(Regex::new(r"^1\d*$").unwrap());
    }
}

#[no_mangle]
pub fn run(n: i32) -> i32 {
    let s = format!("{}", n);
    let regex = unsafe { REGEX.as_ref().unwrap() };
    if regex.is_match(&s) {
        42
    } else {
        0
    }
}
