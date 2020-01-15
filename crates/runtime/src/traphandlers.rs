//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::trap_registry::get_trap_registry;
use crate::trap_registry::TrapDescription;
use crate::vmcontext::{VMContext, VMFunctionBody};
use backtrace::Backtrace;
use std::cell::Cell;
use std::fmt;
use std::ptr;
use wasmtime_environ::ir;

extern "C" {
    fn WasmtimeCallTrampoline(
        vmctx: *mut u8,
        callee: *const VMFunctionBody,
        values_vec: *mut u8,
    ) -> i32;
    fn WasmtimeCall(vmctx: *mut u8, callee: *const VMFunctionBody) -> i32;
}

thread_local! {
    static RECORDED_TRAP: Cell<Option<Trap>> = Cell::new(None);
    static JMP_BUF: Cell<*const u8> = Cell::new(ptr::null());
    static RESET_GUARD_PAGE: Cell<bool> = Cell::new(false);
}

/// Check if there is a trap at given PC
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn CheckIfTrapAtAddress(_pc: *const u8) -> i8 {
    // TODO: stack overflow can happen at any random time (i.e. in malloc() in memory.grow)
    // and it's really hard to determine if the cause was stack overflow and if it happened
    // in WebAssembly module.
    // So, let's assume that any untrusted code called from WebAssembly doesn't trap.
    // Then, if we have called some WebAssembly code, it means the trap is stack overflow.
    JMP_BUF.with(|ptr| !ptr.get().is_null()) as i8
}

/// Record the Trap code and wasm bytecode offset in TLS somewhere
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RecordTrap(pc: *const u8, reset_guard_page: bool) {
    // TODO: please see explanation in CheckIfTrapAtAddress.
    let registry = get_trap_registry();
    let trap = Trap {
        desc: registry
            .get_trap(pc as usize)
            .unwrap_or_else(|| TrapDescription {
                source_loc: ir::SourceLoc::default(),
                trap_code: ir::TrapCode::StackOverflow,
            }),
        backtrace: Backtrace::new_unresolved(),
    };

    if reset_guard_page {
        RESET_GUARD_PAGE.with(|v| v.set(true));
    }

    RECORDED_TRAP.with(|data| {
        let prev = data.replace(Some(trap));
        assert!(
            prev.is_none(),
            "Only one trap per thread can be recorded at a moment!"
        );
    });
}

#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn EnterScope(ptr: *const u8) -> *const u8 {
    JMP_BUF.with(|buf| buf.replace(ptr))
}

#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn GetScope() -> *const u8 {
    JMP_BUF.with(|buf| buf.get())
}

#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn LeaveScope(ptr: *const u8) {
    RESET_GUARD_PAGE.with(|v| {
        if v.get() {
            reset_guard_page();
            v.set(false);
        }
    });

    JMP_BUF.with(|buf| buf.set(ptr))
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
pub struct Trap {
    /// What sort of trap happened, as well as where in the original wasm module
    /// it happened.
    pub desc: TrapDescription,
    /// Native stack backtrace at the time the trap occurred
    pub backtrace: Backtrace,
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "wasm trap: {}, source location: {}",
            trap_code_to_expected_string(self.desc.trap_code),
            self.desc.source_loc
        )
    }
}

impl std::error::Error for Trap {}

fn last_trap() -> Trap {
    RECORDED_TRAP
        .with(|data| data.replace(None))
        .expect("trap_message must be called after trap occurred")
}

fn trap_code_to_expected_string(trap_code: ir::TrapCode) -> String {
    use ir::TrapCode::*;
    match trap_code {
        StackOverflow => "call stack exhausted".to_string(),
        HeapOutOfBounds => "out of bounds memory access".to_string(),
        TableOutOfBounds => "undefined element".to_string(),
        OutOfBounds => "out of bounds".to_string(), // Note: not covered by the test suite
        IndirectCallToNull => "uninitialized element".to_string(),
        BadSignature => "indirect call type mismatch".to_string(),
        IntegerOverflow => "integer overflow".to_string(),
        IntegerDivisionByZero => "integer divide by zero".to_string(),
        BadConversionToInteger => "invalid conversion to integer".to_string(),
        UnreachableCodeReached => "unreachable".to_string(),
        Interrupt => "interrupt".to_string(), // Note: not covered by the test suite
        User(x) => format!("user trap {}", x), // Note: not covered by the test suite
    }
}

/// Call the wasm function pointed to by `callee`. `values_vec` points to
/// a buffer which holds the incoming arguments, and to which the outgoing
/// return values will be written.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call_trampoline(
    vmctx: *mut VMContext,
    callee: *const VMFunctionBody,
    values_vec: *mut u8,
) -> Result<(), Trap> {
    if WasmtimeCallTrampoline(vmctx as *mut u8, callee, values_vec) == 0 {
        Err(last_trap())
    } else {
        Ok(())
    }
}

/// Call the wasm function pointed to by `callee`, which has no arguments or
/// return values.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call(
    vmctx: *mut VMContext,
    callee: *const VMFunctionBody,
) -> Result<(), Trap> {
    if WasmtimeCall(vmctx as *mut u8, callee) == 0 {
        Err(last_trap())
    } else {
        Ok(())
    }
}
