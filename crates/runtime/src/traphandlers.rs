//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::trap_registry::get_trap_registry;
use crate::trap_registry::TrapDescription;
use crate::vmcontext::{VMContext, VMFunctionBody};
use backtrace::Backtrace;
use std::any::Any;
use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::ptr;
use wasmtime_environ::ir;

extern "C" {
    fn WasmtimeCallTrampoline(
        jmp_buf: *mut *const u8,
        vmctx: *mut u8,
        caller_vmctx: *mut u8,
        callee: *const VMFunctionBody,
        values_vec: *mut u8,
    ) -> i32;
    fn WasmtimeCall(
        jmp_buf: *mut *const u8,
        vmctx: *mut u8,
        caller_vmctx: *mut u8,
        callee: *const VMFunctionBody,
    ) -> i32;
    fn Unwind(jmp_buf: *const u8) -> !;
}

/// Record the Trap code and wasm bytecode offset in TLS somewhere
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RecordTrap(pc: *const u8, reset_guard_page: bool) -> *const u8 {
    tls::with(|info| {
        // TODO: stack overflow can happen at any random time (i.e. in malloc()
        // in memory.grow) and it's really hard to determine if the cause was
        // stack overflow and if it happened in WebAssembly module.
        //
        // So, let's assume that any untrusted code called from WebAssembly
        // doesn't trap. Then, if we have called some WebAssembly code, it
        // means the trap is stack overflow.
        if info.jmp_buf.get().is_null() {
            return ptr::null();
        }

        let registry = get_trap_registry();
        let trap = Trap::Wasm {
            desc: registry
                .get_trap(pc as usize)
                .unwrap_or_else(|| TrapDescription {
                    source_loc: ir::SourceLoc::default(),
                    trap_code: ir::TrapCode::StackOverflow,
                }),
            backtrace: Backtrace::new_unresolved(),
        };

        if reset_guard_page {
            info.reset_guard_page.set(true);
        }

        info.unwind.replace(UnwindReason::Trap(trap));
        info.jmp_buf.get()
    })
}

/// Raises a user-defined trap immediately.
///
/// This function performs as-if a wasm trap was just executed, only the trap
/// has a dynamic payload associated with it which is user-provided. This trap
/// payload is then returned from `wasmtime_call` an `wasmtime_call_trampoline`
/// below.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `wasmtime_call` or
/// `wasmtime_call_trampoline` must have been previously called.
pub unsafe fn raise_user_trap(data: Box<dyn Error + Send + Sync>) -> ! {
    let trap = Trap::User(data);
    tls::with(|info| info.unwind_with(UnwindReason::Trap(trap)))
}

/// Carries a Rust panic across wasm code and resumes the panic on the other
/// side.
///
/// # Safety
///
/// Only safe to call when wasm code is on the stack, aka `wasmtime_call` or
/// `wasmtime_call_trampoline` must have been previously called.
pub unsafe fn resume_panic(payload: Box<dyn Any + Send>) -> ! {
    tls::with(|info| info.unwind_with(UnwindReason::Panic(payload)))
}

#[cfg(target_os = "windows")]
fn reset_guard_page() {
    extern "C" {
        fn _resetstkoflw() -> winapi::ctypes::c_int;
    }

    // We need to restore guard page under stack to handle future stack overflows properly.
    // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/resetstkoflw?view=vs-2019
    if unsafe { _resetstkoflw() } == 0 {
        panic!("failed to restore stack guard page");
    }
}

#[cfg(not(target_os = "windows"))]
fn reset_guard_page() {}

/// Stores trace message with backtrace.
#[derive(Debug)]
pub enum Trap {
    /// A user-raised trap through `raise_user_trap`.
    User(Box<dyn Error + Send + Sync>),
    /// A wasm-originating trap from wasm code itself.
    Wasm {
        /// What sort of trap happened, as well as where in the original wasm module
        /// it happened.
        desc: TrapDescription,
        /// Native stack backtrace at the time the trap occurred
        backtrace: Backtrace,
    },
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trap::User(user) => user.fmt(f),
            Trap::Wasm { desc, .. } => desc.fmt(f),
        }
    }
}

impl std::error::Error for Trap {}

/// Call the wasm function pointed to by `callee`. `values_vec` points to
/// a buffer which holds the incoming arguments, and to which the outgoing
/// return values will be written.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call_trampoline(
    vmctx: *mut VMContext,
    caller_vmctx: *mut VMContext,
    callee: *const VMFunctionBody,
    values_vec: *mut u8,
) -> Result<(), Trap> {
    let cx = CallThreadState::new();
    let ret = tls::set(&cx, || {
        WasmtimeCallTrampoline(
            cx.jmp_buf.as_ptr(),
            vmctx as *mut u8,
            caller_vmctx as *mut u8,
            callee,
            values_vec,
        )
    });
    cx.into_result(ret)
}

/// Call the wasm function pointed to by `callee`, which has no arguments or
/// return values.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call(
    vmctx: *mut VMContext,
    caller_vmctx: *mut VMContext,
    callee: *const VMFunctionBody,
) -> Result<(), Trap> {
    let cx = CallThreadState::new();
    let ret = tls::set(&cx, || {
        WasmtimeCall(
            cx.jmp_buf.as_ptr(),
            vmctx as *mut u8,
            caller_vmctx as *mut u8,
            callee,
        )
    });
    cx.into_result(ret)
}

/// Temporary state stored on the stack which is registered in the `tls` module
/// below for calls into wasm.
pub struct CallThreadState {
    unwind: Cell<UnwindReason>,
    jmp_buf: Cell<*const u8>,
    reset_guard_page: Cell<bool>,
}

enum UnwindReason {
    None,
    Panic(Box<dyn Any + Send>),
    Trap(Trap),
}

impl CallThreadState {
    fn new() -> CallThreadState {
        CallThreadState {
            unwind: Cell::new(UnwindReason::None),
            jmp_buf: Cell::new(ptr::null()),
            reset_guard_page: Cell::new(false),
        }
    }

    fn into_result(self, ret: i32) -> Result<(), Trap> {
        match self.unwind.replace(UnwindReason::None) {
            UnwindReason::None => {
                debug_assert_eq!(ret, 1);
                Ok(())
            }
            UnwindReason::Trap(trap) => {
                debug_assert_eq!(ret, 0);
                Err(trap)
            }
            UnwindReason::Panic(panic) => {
                debug_assert_eq!(ret, 0);
                std::panic::resume_unwind(panic)
            }
        }
    }

    fn unwind_with(&self, reason: UnwindReason) -> ! {
        self.unwind.replace(reason);
        unsafe {
            Unwind(self.jmp_buf.get());
        }
    }
}

impl Drop for CallThreadState {
    fn drop(&mut self) {
        if self.reset_guard_page.get() {
            reset_guard_page();
        }
    }
}

// A private inner module for managing the TLS state that we require across
// calls in wasm. The WebAssembly code is called from C++ and then a trap may
// happen which requires us to read some contextual state to figure out what to
// do with the trap. This `tls` module is used to persist that information from
// the caller to the trap site.
mod tls {
    use super::CallThreadState;
    use std::cell::Cell;
    use std::ptr;

    thread_local!(static PTR: Cell<*const CallThreadState> = Cell::new(ptr::null()));

    /// Configures thread local state such that for the duration of the
    /// execution of `closure` any call to `with` will yield `ptr`, unless this
    /// is recursively called again.
    pub fn set<R>(ptr: &CallThreadState, closure: impl FnOnce() -> R) -> R {
        struct Reset<'a, T: Copy>(&'a Cell<T>, T);

        impl<T: Copy> Drop for Reset<'_, T> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }

        PTR.with(|p| {
            let _r = Reset(p, p.replace(ptr));
            closure()
        })
    }

    /// Returns the last pointer configured with `set` above. Panics if `set`
    /// has not been previously called.
    pub fn with<R>(closure: impl FnOnce(&CallThreadState) -> R) -> R {
        PTR.with(|ptr| {
            let p = ptr.get();
            assert!(!p.is_null());
            unsafe { closure(&*p) }
        })
    }
}
