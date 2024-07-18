// For MIRI, set up just enough of a setjmp/longjmp with catching panics
// to get a few tests working that use this.
//
// Note that no actual JIT code runs in MIRI so this is purely here for
// host-to-host calls.

use crate::runtime::vm::VMContext;

struct WasmtimeLongjmp;

pub fn wasmtime_setjmp(
    _jmp_buf: *mut *const u8,
    callback: extern "C" fn(*mut u8, *mut VMContext),
    payload: *mut u8,
    callee: *mut VMContext,
) -> i32 {
    use std::panic::{self, AssertUnwindSafe};
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        callback(payload, callee);
    }));
    match result {
        Ok(()) => 1,
        Err(e) => {
            if e.is::<WasmtimeLongjmp>() {
                0
            } else {
                panic::resume_unwind(e)
            }
        }
    }
}

pub fn wasmtime_longjmp(_jmp_buf: *const u8) -> ! {
    std::panic::panic_any(WasmtimeLongjmp)
}

#[allow(missing_docs)]
pub type SignalHandler<'a> = dyn Fn() + Send + Sync + 'a;

pub unsafe fn platform_init(_macos_use_mach_ports: bool) {}

pub fn lazy_per_thread_init() {}

#[cfg(target_os = "macos")]
pub fn using_mach_ports() -> bool {
    false
}
