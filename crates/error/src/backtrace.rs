use std::backtrace::Backtrace;
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);

fn enabled() -> bool {
    ENABLED.load(Ordering::Acquire)
}

/// Forcibly disable capturing backtraces dynamically.
///
/// XXX: This is only exposed for internal testing, to work around cargo
/// workspaces and feature resolution. This method may disappear or change
/// at any time. Instead of using this method, you should disable the
/// `backtrace` cargo feature.
#[doc(hidden)]
pub fn disable_backtrace() {
    ENABLED.store(false, Ordering::Release)
}

#[track_caller]
pub fn capture() -> Backtrace {
    if enabled() {
        Backtrace::capture()
    } else {
        Backtrace::disabled()
    }
}
