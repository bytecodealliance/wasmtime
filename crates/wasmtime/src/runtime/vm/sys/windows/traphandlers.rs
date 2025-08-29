pub fn lazy_per_thread_init() {
    // unused on Windows
}

cfg_if::cfg_if! {
    if #[cfg(has_native_signals)] {
        pub use super::vectored_exceptions::{TrapHandler, SignalHandler };
    } else {
        pub enum SignalHandler {}
    }
}
