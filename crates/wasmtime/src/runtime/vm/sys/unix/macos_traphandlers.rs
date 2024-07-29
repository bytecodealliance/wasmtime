/// Whether or not macOS is using mach ports.
#[cfg(target_os = "macos")]
static mut USE_MACH_PORTS: bool = false;

pub use super::signals::{wasmtime_longjmp, wasmtime_setjmp, SignalHandler};

pub enum TrapHandler {
    Signals(super::signals::TrapHandler),
    #[allow(dead_code)] // used for its drop
    MachPorts(super::machports::TrapHandler),
}

impl TrapHandler {
    pub unsafe fn new(macos_use_mach_ports: bool) -> TrapHandler {
        USE_MACH_PORTS = macos_use_mach_ports;
        if macos_use_mach_ports {
            TrapHandler::MachPorts(super::machports::TrapHandler::new())
        } else {
            TrapHandler::Signals(super::signals::TrapHandler::new(false))
        }
    }

    pub fn validate_config(&self, macos_use_mach_ports: bool) {
        match self {
            TrapHandler::Signals(t) => t.validate_config(macos_use_mach_ports),
            TrapHandler::MachPorts(_) => assert!(macos_use_mach_ports),
        }
    }
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
