//! Functionality to convert signals to synthetic calls.
//!
//! When handling a signal that occurs *synchronously*, i.e.,
//! intentionally as a result of an instruction in generated code, we
//! sometimes wish to convert the signal to behavior as-if the
//! faulting instruction had been a call. This may seem somewhat odd,
//! but it comes in extremely useful for:
//!
//! - Allowing introspection of trapping state when a trap is detected
//!   as a caught signal (e.g. an out-of-bounds Wasm memory access)
//!   before the stack is unwound (or fiber stack is thrown away).
//!
//! - Allowing "resumable traps" for debugging breakpoints: a trap
//!   instruction (e.g. `ud2` on x86-64 or `brk` on aarch64) can be
//!   converted to a "yield to debugger" hostcall, and we can even
//!   return and continue by incrementing PC beyond the instruction.
//!
//! We do not require space on the stack, and we don't inject a stack
//! frame (because Windows vectored exception handling won't allow us
//! to do so: the exception handler runs on the same stack already, so
//! its frame is in the way). We also have to preserve all register
//! state, because we don't want to treat all potentially-trapping
//! instructions as clobbering a bunch of registers. Rather, our
//! trampoline should save all state.
//!
//! Because we're injecting a new state value on trap (a new PC, to
//! our trampoline), and because we need to save all existing state,
//! something has to give: we don't have anywhere in the register
//! state to save the existing PC of the trap location. Instead, what
//! we do is we make use of the fact that we have access to the Store
//! here: we save off the trapping PC, and just overwrite PC in the
//! signal-frame context to redirect. The trampoline will then call
//! into host code and that host code will return the original PC; the
//! trampoline will restore all state it saved on the stack, and
//! resume to that PC.

use crate::vm::VMStoreContext;

/// State for a hostcall invocation created in response to a signal.
///
/// This state is created when we perform the state mutation within
/// the signal handler, and is consumed when we return to the injected
/// trampoline so that it can resume to the original code.
pub struct InjectedCallState {
    /// Saved PC.
    pub pc: usize,
    /// Saved first argument register.
    arg0: usize,
    /// Saved second argument register.
    arg1: usize,
}

impl InjectedCallState {
    fn inject(
        pc: &mut usize,
        arg0: &mut usize,
        arg1: &mut usize,
        trampoline: usize,
        trampoline_arg: usize,
    ) -> InjectedCallState {
        assert!(SUPPORTED_ARCH);

        log::trace!(
            "inject: orig pc {:x} at stack slot {:x}; injecting {:x}",
            *pc,
            pc as *const _ as usize,
            injected_call_trampoline as usize
        );
        // Save off the PC at the trapping location, and update it to
        // point to our trampoline.
        let orig_pc = core::mem::replace(pc, injected_call_trampoline as usize);
        let saved_arg0 = core::mem::replace(arg0, trampoline);
        let saved_arg1 = core::mem::replace(arg1, trampoline_arg);
        // Save the original state to restore in the stack frame while
        // the injected call runs.
        InjectedCallState {
            // Note: we don't yet support resumable traps, but that
            // will be implemented soon; when it is, then we will
            // actually return to this PC.
            pc: orig_pc,
            arg0: saved_arg0,
            arg1: saved_arg1,
        }
    }

    fn restore(self, pc: &mut usize, arg0: &mut usize, arg1: &mut usize) {
        log::trace!(
            "restore: pc slot at {:x} gets {:x}",
            pc as *const _ as usize,
            self.pc
        );
        *pc = self.pc;
        *arg0 = self.arg0;
        *arg1 = self.arg1;
    }
}

impl VMStoreContext {
    /// From a VMStoreContext in a trap context, inject a call to the
    /// trap-handler hostcall.
    ///
    /// `pc`, `arg0`, and `arg1` are mutable borrows to register
    /// values in the signal register context corresponding to the
    /// program counter and first and second function argument
    /// registers on the current platform.
    pub(crate) fn inject_trap_handler_hostcall(
        &mut self,
        pc: &mut usize,
        arg0: &mut usize,
        arg1: &mut usize,
    ) {
        let vmctx_raw_ptr = self as *mut _ as usize;
        let handler = injected_trap_handler as usize;
        let state = InjectedCallState::inject(pc, arg0, arg1, handler, vmctx_raw_ptr);
        let old = core::mem::replace(self.injected_call_state.get_mut(), Some(state));
        assert!(old.is_none());
    }

    /// From the trap-handler hostcall, restore state needed to return
    /// from the hostcall.
    ///
    /// `pc`, `arg0`, and `arg1` are mutable borrows to register
    /// values in the tramopline's register-save frame to be updated.
    pub(crate) fn trap_handler_hostcall_fixup(
        &mut self,
        pc: &mut usize,
        arg0: &mut usize,
        arg1: &mut usize,
    ) {
        let state = core::mem::replace(self.injected_call_state.get_mut(), None)
            .expect("Saved register state must be present");
        state.restore(pc, arg0, arg1);
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        mod aarch64;
        pub const SUPPORTED_ARCH: bool = true;
        pub(crate) use aarch64::*;
    } else if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        pub const SUPPORTED_ARCH: bool = true;
        pub(crate) use x86_64::*;
    } else if #[cfg(target_arch = "s390x")] {
        mod s390x;
        pub const SUPPORTED_ARCH: bool = true;
        pub(crate) use s390x::*;
    } else if #[cfg(target_arch = "riscv64")]  {
        mod riscv64;
        pub const SUPPORTED_ARCH: bool = true;
        pub(crate) use riscv64::*;
    } else {
        pub(crate) use unsupported::*;
    }
}

#[allow(
    dead_code,
    reason = "expected to have dead code in some configurations"
)]
mod unsupported {
    pub const SUPPORTED_ARCH: bool = false;
    pub(crate) fn injected_call_trampoline() -> ! {
        unreachable!()
    }
}

/// This handler is invoked directly from the stub injected by the
/// signal handler on Wasm code when debugging is enabled; from its
/// perspective, it has been called in a Wasm context, i.e. without a
/// normal VM exit trampoline. The debug yield reason will have
/// already been filled in; this handler's only job is to restore the
/// register state used to inject the stub call, then suspend the
/// fiber, and then return when it resumes.
unsafe extern "C" fn injected_trap_handler(
    vm_store_context: *mut VMStoreContext,
    orig_pc: *mut u8,
    orig_arg0: *mut u8,
    orig_arg1: *mut u8,
) {
    log::trace!(
        "injected trap handler running; orig_pc = {:x}",
        orig_pc as usize
    );
    let vm_store_context = unsafe {
        vm_store_context
            .as_mut()
            .expect("null VMStoreContext pointer")
    };
    let orig_pc: &mut usize = unsafe { &mut *orig_pc.cast::<usize>() };
    let orig_arg0: &mut usize = unsafe { &mut *orig_arg0.cast::<usize>() };
    let orig_arg1: &mut usize = unsafe { &mut *orig_arg1.cast::<usize>() };
    vm_store_context.trap_handler_hostcall_fixup(orig_pc, orig_arg0, orig_arg1);

    // SAFETY: we have a valid VMStoreContext, so we can use its Store
    // backpointer. This will be the only store reference held in this
    // context so it is valid to derive.
    let store = unsafe { vm_store_context.raw_store_mut() };
    super::tls::with(|s| {
        let s = s.expect("Trap context must have a CallThreadState");
        s.invoke_debug_event(store);
    });

    // Now perform the original trap resume to the entry trap
    // handler. Note: once we support resumable traps, this is where
    // we will instead update `orig_pc` to skip over a break
    // instruction and return if resuming.

    // SAFETY: we are in a trap context, and all entries into Wasm
    // that would have trapped will set an entry trap handler.
    unsafe {
        injected_trap_terminate();
    }
}

/// Perform the original resumption to the entry trap resume point.
///
/// # Safety
///
/// We must be in a trap context with a valid entry trap handler ste.
unsafe extern "C" fn injected_trap_terminate() -> ! {
    let handler = super::tls::with(|state| {
        let state = state.expect("there must be an active CallThreadState");
        state.entry_trap_handler()
    });
    log::trace!("injected_trap_terminate about to invoke original entry handler");
    unsafe { handler.resume_tailcc(0, 0) }
}

const _: () = {
    #[used]
    static USED1: unsafe extern "C" fn(*mut VMStoreContext, *mut u8, *mut u8, *mut u8) =
        injected_trap_handler;
    #[used]
    static USED2: unsafe extern "C" fn() -> ! = injected_trap_terminate;
};
