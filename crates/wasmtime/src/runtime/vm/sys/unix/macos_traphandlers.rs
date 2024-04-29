/// Whether or not macOS is using mach ports.
#[cfg(target_os = "macos")]
static mut USE_MACH_PORTS: bool = false;

pub use super::signals::{wasmtime_longjmp, wasmtime_setjmp, SignalHandler};

pub unsafe fn platform_init(macos_use_mach_ports: bool) {
    USE_MACH_PORTS = macos_use_mach_ports;
    if macos_use_mach_ports {
        super::machports::platform_init();
    } else {
        super::signals::platform_init(false);
    }
}

pub fn using_mach_ports() -> bool {
    unsafe { USE_MACH_PORTS }
}

pub fn lazy_per_thread_init() {
    unsafe {
        if USE_MACH_PORTS {
            super::machports::lazy_per_thread_init();
        } else {
            super::signals::lazy_per_thread_init();
        }
    }
}
