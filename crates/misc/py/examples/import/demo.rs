extern "C" {
    fn callback(s: *const u8, s_len: u32) -> u32;
}

static MSG: &str = "Hello, world!";

#[no_mangle]
pub extern "C" fn test() {
    unsafe {
        callback(MSG.as_ptr(), MSG.len() as u32);
    }
}
