//! Trap handling support when `feature = "signals-based-traps"` is enabled.
//!
//! This module is conditionally included in the above `traphandlers` module and
//! contains support and shared routines for working with signals-based traps.
//! Each platform will have its own `signals-based-traps` configuration and
//! thise module serves as a shared entrypoint for initialization entrypoints
//! (`init_traps`) and testing if a trapping opcode is wasm (`test_if_trap`).

use crate::runtime::module::lookup_code;
use crate::sync::RwLock;
use crate::vm::sys::traphandlers::TrapHandler;
use crate::vm::traphandlers::{CallThreadState, SignalHandler, TrapReason, UnwindReason};
use core::ptr;

pub(crate) struct TrapRegisters {
    pub pc: usize,
    pub fp: usize,
}

/// Return value from `test_if_trap`.
pub(crate) enum TrapTest {
    /// Not a wasm trap, need to delegate to whatever process handler is next.
    NotWasm,
    /// This trap was handled by the embedder via custom embedding APIs.
    HandledByEmbedder,
    /// This is a wasm trap, it needs to be handled.
    #[cfg_attr(miri, allow(dead_code))]
    Trap {
        /// How to longjmp back to the original wasm frame.
        jmp_buf: *const u8,
    },
}

/// Platform-specific trap-handler state.
///
/// This state is protected by a lock to synchronize access to it. Right now
/// it's a `RwLock` but it could be a `Mutex`, and `RwLock` is just chosen for
/// convenience as it's what's implemented in no_std. The performance here
/// should not be of consequence.
///
/// This is initialized to `None` and then set as part of `init_traps`.
static TRAP_HANDLER: RwLock<Option<TrapHandler>> = RwLock::new(None);

/// This function is required to be called before any WebAssembly is entered.
/// This will configure global state such as signal handlers to prepare the
/// process to receive wasm traps.
///
/// # Panics
///
/// This function will panic on macOS if it is called twice or more times with
/// different values of `macos_use_mach_ports`.
///
/// This function will also panic if the `std` feature is disabled and it's
/// called concurrently.
pub fn init_traps(macos_use_mach_ports: bool) {
    let mut lock = TRAP_HANDLER.write();
    match lock.as_mut() {
        Some(state) => state.validate_config(macos_use_mach_ports),
        None => *lock = Some(unsafe { TrapHandler::new(macos_use_mach_ports) }),
    }
}

/// De-initializes platform-specific state for trap handling.
///
/// # Panics
///
/// This function will also panic if the `std` feature is disabled and it's
/// called concurrently.
///
/// # Aborts
///
/// This may abort the process on some platforms where trap handling state
/// cannot be unloaded.
///
/// # Unsafety
///
/// This is not safe to be called unless all wasm code is unloaded. This is not
/// safe to be called on some platforms, like Unix, when other libraries
/// installed their own signal handlers after `init_traps` was called.
///
/// There's more reasons for unsafety here than those articulated above,
/// generally this can only be called "if you know what you're doing".
pub unsafe fn deinit_traps() {
    let mut lock = TRAP_HANDLER.write();
    let _ = lock.take();
}

impl CallThreadState {
    /// Trap handler using our thread-local state.
    ///
    /// * `pc` - the program counter the trap happened at
    /// * `call_handler` - a closure used to invoke the platform-specific
    ///   signal handler for each instance, if available.
    ///
    /// Attempts to handle the trap if it's a wasm trap. Returns a few
    /// different things:
    ///
    /// * null - the trap didn't look like a wasm trap and should continue as a
    ///   trap
    /// * 1 as a pointer - the trap was handled by a custom trap handler on an
    ///   instance, and the trap handler should quickly return.
    /// * a different pointer - a jmp_buf buffer to longjmp to, meaning that
    ///   the wasm trap was successfully handled.
    pub(crate) fn test_if_trap(
        &self,
        regs: TrapRegisters,
        faulting_addr: Option<usize>,
        call_handler: impl Fn(&SignalHandler) -> bool,
    ) -> TrapTest {
        // If we haven't even started to handle traps yet, bail out.
        if self.jmp_buf.get().is_null() {
            return TrapTest::NotWasm;
        }

        // First up see if any instance registered has a custom trap handler,
        // in which case run them all. If anything handles the trap then we
        // return that the trap was handled.
        if let Some(handler) = self.signal_handler {
            if unsafe { call_handler(&*handler) } {
                return TrapTest::HandledByEmbedder;
            }
        }

        // If this fault wasn't in wasm code, then it's not our problem
        let Some((code, text_offset)) = lookup_code(regs.pc) else {
            return TrapTest::NotWasm;
        };

        let Some(trap) = code.lookup_trap_code(text_offset) else {
            return TrapTest::NotWasm;
        };

        self.set_jit_trap(regs, faulting_addr, trap);

        // If all that passed then this is indeed a wasm trap, so return the
        // `jmp_buf` passed to `wasmtime_longjmp` to resume.
        TrapTest::Trap {
            jmp_buf: self.take_jmp_buf(),
        }
    }

    pub(crate) fn take_jmp_buf(&self) -> *const u8 {
        self.jmp_buf.replace(ptr::null())
    }

    pub(crate) fn set_jit_trap(
        &self,
        TrapRegisters { pc, fp, .. }: TrapRegisters,
        faulting_addr: Option<usize>,
        trap: wasmtime_environ::Trap,
    ) {
        let backtrace = self.capture_backtrace(self.limits, Some((pc, fp)));
        let coredump = self.capture_coredump(self.limits, Some((pc, fp)));
        self.unwind.set(Some((
            UnwindReason::Trap(TrapReason::Jit {
                pc,
                faulting_addr,
                trap,
            }),
            backtrace,
            coredump,
        )))
    }
}
