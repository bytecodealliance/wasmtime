use regex::Regex;

/// A regex that matches numbers that start with "1".
static mut REGEX: Option<Regex> = None;

#[export_name = "wizer.initialize"]
pub extern "C" fn init() {
    unsafe {
        REGEX = Some(Regex::new(r"^1\d*$").unwrap());
    }
}

#[export_name = "run"]
pub extern "C" fn run(ptr: *mut u8, len: usize) -> i32 {
    #[cfg(not(feature = "wizer"))]
    init();

    let s = unsafe {
        let slice = std::slice::from_raw_parts(ptr, len);
        std::str::from_utf8(slice).unwrap()
    };
    let regex = unsafe { REGEX.as_ref().unwrap() };
    regex.is_match(&s) as u8 as i32
}
