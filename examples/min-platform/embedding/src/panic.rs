#![cfg(not(unix))] // gets `cargo clippy` working

use core::panic::PanicInfo;

#[panic_handler]
fn handler(_info: &PanicInfo) -> ! {
    // NB: should ideally print something here but for this example this is left
    // out. A more complete embedding would likely turn `info` into a
    // stack-allocated string and then pass that as a message to the outer
    // system to get printed and trigger a failure.
    //
    // Here an illegal instruction is executed, where possible, to abort the
    // process so that failures don't manifest as hangs.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("ud2", options(noreturn));
    }
    #[cfg(not(target_arch = "x86_64"))]
    loop {}
}
