use core::panic::PanicInfo;

#[panic_handler]
fn handler(_info: &PanicInfo) -> ! {
    // NB: should ideally print something here but for this example this is left
    // out. A more complete embedding would likely turn `info` into a
    // stack-allocated string and then pass that as a message to the outer
    // system to get printed and trigger a failure.
    loop {}
}
