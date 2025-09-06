cfg_if::cfg_if! {
    if #[cfg(not(has_native_signals))] {
        // If signals-based traps are disabled statically then there's no
        // platform signal handler and no per-thread init, so stub these both
        // out.
        pub enum SignalHandler {}

        #[inline]
        pub fn lazy_per_thread_init() {}
    } else if #[cfg(target_vendor = "apple")] {
        // On macOS a dynamic decision is made to use mach ports or signals at
        // process initialization time.

        /// Whether or not macOS is using mach ports.
        static mut USE_MACH_PORTS: bool = false;

        pub use super::signals::SignalHandler;

        pub enum TrapHandler {
            Signals(super::signals::TrapHandler),
            #[allow(dead_code)] // used for its drop
            MachPorts(super::machports::TrapHandler),
        }

        impl TrapHandler {
            pub unsafe fn new(macos_use_mach_ports: bool) -> TrapHandler {
                unsafe {
                    USE_MACH_PORTS = macos_use_mach_ports;
                    if macos_use_mach_ports {
                        TrapHandler::MachPorts(super::machports::TrapHandler::new())
                    } else {
                        TrapHandler::Signals(super::signals::TrapHandler::new(false))
                    }
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
    } else {
        // Otherwise unix platforms use the signals-based implementation of
        // these functions.
        pub use super::signals::{TrapHandler, SignalHandler, lazy_per_thread_init};
    }
}
