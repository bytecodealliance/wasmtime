//! Interface to low-level signal-handling mechanisms.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::borrow::{Borrow, BorrowMut};
use std::sync::RwLock;

include!(concat!(env!("OUT_DIR"), "/signalhandlers.rs"));

struct InstallState {
    tried: bool,
    success: bool,
}

impl InstallState {
    fn new() -> Self {
        Self {
            tried: false,
            success: false,
        }
    }
}

lazy_static! {
    static ref EAGER_INSTALL_STATE: RwLock<InstallState> = RwLock::new(InstallState::new());
    static ref LAZY_INSTALL_STATE: RwLock<InstallState> = RwLock::new(InstallState::new());
}

/// This function performs the low-overhead signal handler initialization that we
/// want to do eagerly to ensure a more-deterministic global process state. This
/// is especially relevant for signal handlers since handler ordering depends on
/// installation order: the wasm signal handler must run *before* the other crash
/// handlers and since POSIX signal handlers work LIFO, this function needs to be
/// called at the end of the startup process, after other handlers have been
/// installed. This function can thus be called multiple times, having no effect
/// after the first call.
pub fn ensure_eager_signal_handlers() {
    let mut locked = EAGER_INSTALL_STATE.write().unwrap();
    let state = locked.borrow_mut();

    if state.tried {
        return;
    }

    state.tried = true;
    assert!(!state.success);

    if !unsafe { EnsureEagerSignalHandlers() } {
        return;
    }

    state.success = true;
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn ensure_darwin_mach_ports() {
    let mut locked = LAZY_INSTALL_STATE.write().unwrap();
    let state = locked.borrow_mut();

    if state.tried {
        return;
    }

    state.tried = true;
    assert!(!state.success);

    if !unsafe { EnsureDarwinMachPorts() } {
        return;
    }

    state.success = true;
}

/// Assuming `EnsureEagerProcessSignalHandlers` has already been called,
/// this function performs the full installation of signal handlers which must
/// be performed per-thread. This operation may incur some overhead and
/// so should be done only when needed to use wasm.
pub fn ensure_full_signal_handlers(cx: &mut TrapContext) {
    if cx.triedToInstallSignalHandlers {
        return;
    }

    cx.triedToInstallSignalHandlers = true;
    assert!(!cx.haveSignalHandlers);

    {
        let locked = EAGER_INSTALL_STATE.read().unwrap();
        let state = locked.borrow();
        assert!(state.tried);
        if !state.success {
            return;
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    ensure_darwin_mach_ports();

    cx.haveSignalHandlers = true;
}
