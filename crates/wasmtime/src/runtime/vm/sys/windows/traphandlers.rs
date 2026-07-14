pub fn lazy_per_thread_init() {
    // unused on Windows
}

cfg_select! {
    has_native_signals => {
        pub use super::vectored_exceptions::{TrapHandler, SignalHandler };
    }
    _ => {
        pub enum SignalHandler {}
    }
}
