//! Interface to low-level signal-handling mechanisms.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::vmcontext::VMContext;
use lazy_static::lazy_static;
use std::borrow::{Borrow, BorrowMut};
use std::cell::Cell;
use std::sync::RwLock;

#[derive(Default)]
struct TrapContext {
    tried_to_install_signal_handlers: Cell<bool>,
    have_signal_handlers: Cell<bool>,
}

extern "C" {
    fn EnsureEagerSignalHandlers() -> libc::c_int;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn EnsureDarwinMachPorts() -> libc::c_int;
}

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
#[no_mangle]
pub extern "C" fn wasmtime_init_eager() {
    let mut locked = EAGER_INSTALL_STATE.write().unwrap();
    let state = locked.borrow_mut();

    if state.tried {
        return;
    }

    state.tried = true;
    assert!(!state.success);

    if unsafe { EnsureEagerSignalHandlers() == 0 } {
        return;
    }

    state.success = true;
}

thread_local! {
    static TRAP_CONTEXT: TrapContext = TrapContext::default();
}

/// Assuming `EnsureEagerProcessSignalHandlers` has already been called,
/// this function performs the full installation of signal handlers which must
/// be performed per-thread. This operation may incur some overhead and
/// so should be done only when needed to use wasm.
#[no_mangle]
pub extern "C" fn wasmtime_init_finish(vmctx: &mut VMContext) {
    if !TRAP_CONTEXT.with(|cx| cx.tried_to_install_signal_handlers.get()) {
        TRAP_CONTEXT.with(|cx| {
            cx.tried_to_install_signal_handlers.set(true);
            assert!(!cx.have_signal_handlers.get());
        });

        {
            let locked = EAGER_INSTALL_STATE.read().unwrap();
            let state = locked.borrow();
            assert!(
                state.tried,
                "call wasmtime_init_eager before calling wasmtime_init_finish"
            );
            if !state.success {
                return;
            }
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        ensure_darwin_mach_ports();

        TRAP_CONTEXT.with(|cx| {
            cx.have_signal_handlers.set(true);
        })
    }

    let instance = unsafe { vmctx.instance() };
    let have_signal_handlers = TRAP_CONTEXT.with(|cx| cx.have_signal_handlers.get());
    if !have_signal_handlers && instance.needs_signal_handlers() {
        panic!("failed to install signal handlers");
    }
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

    if unsafe { EnsureDarwinMachPorts() != 0 } {
        return;
    }

    state.success = true;
}
