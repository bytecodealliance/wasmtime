//! Interface to low-level signal-handling mechanisms.

use std::sync::Once;

extern "C" {
    fn EnsureEagerSignalHandlers() -> libc::c_int;
}

/// This function performs the low-overhead signal handler initialization that
/// we want to do eagerly to ensure a more-deterministic global process state.
///
/// This is especially relevant for signal handlers since handler ordering
/// depends on installation order: the wasm signal handler must run *before*
/// the other crash handlers and since POSIX signal handlers work LIFO, this
/// function needs to be called at the end of the startup process, after other
/// handlers have been installed. This function can thus be called multiple
/// times, having no effect after the first call.
pub fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(real_init);
}

fn real_init() {
    // This is a really weird and unfortunate function call. For all the gory
    // details see #829, but the tl;dr; is that in a trap handler we have 2
    // pages of stack space on Linux, and calling into libunwind which triggers
    // the dynamic loader blows the stack.
    //
    // This is a dumb hack to work around this system-specific issue by
    // capturing a backtrace once in the lifetime of a process to ensure that
    // when we capture a backtrace in the trap handler all caches are primed,
    // aka the dynamic loader has resolved all the relevant symbols.
    drop(backtrace::Backtrace::new_unresolved());

    if unsafe { EnsureEagerSignalHandlers() == 0 } {
        panic!("failed to install signal handlers");
    }
}
