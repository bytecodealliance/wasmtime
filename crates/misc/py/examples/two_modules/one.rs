extern "C" {
    fn answer() -> u32;
}

// For the purpose of this wasm example, we don't worry about multi-threading,
// and will be using the PLACE in unsafe manner below.
static mut PLACE: u32 = 23;

#[no_mangle]
pub extern "C" fn bar() -> *const u32 {
    unsafe {
        PLACE = answer();
        // Return a pointer to the exported memory.
        (&PLACE) as *const u32
    }
}
