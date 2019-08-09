//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::vmcontext::{VMContext, VMFunctionBody};
use core::cell::Cell;
use core::ptr;
use std::string::String;

extern "C" {
    fn WasmtimeCallTrampoline(
        vmctx: *mut u8,
        callee: *const VMFunctionBody,
        values_vec: *mut u8,
    ) -> i32;
    fn WasmtimeCall(vmctx: *mut u8, callee: *const VMFunctionBody) -> i32;
}

thread_local! {
    static TRAP_PC: Cell<*const u8> = Cell::new(ptr::null());
    static JMP_BUF: Cell<*const u8> = Cell::new(ptr::null());
}

/// Record the Trap code and wasm bytecode offset in TLS somewhere
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RecordTrap(pc: *const u8) {
    // TODO: Look up the wasm bytecode offset and trap code and record them instead.
    TRAP_PC.with(|data| data.set(pc));
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
    JMP_BUF.with(|buf| buf.set(ptr))
}

fn trap_message(_vmctx: *mut VMContext) -> String {
    let pc = TRAP_PC.with(|data| data.replace(ptr::null()));

    // TODO: Record trap metadata in the VMContext, and look up the
    // pc to obtain the TrapCode and SourceLoc.

    format!("wasm trap at {:?}", pc)
}

/// Call the wasm function pointed to by `callee`. `values_vec` points to
/// a buffer which holds the incoming arguments, and to which the outgoing
/// return values will be written.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call_trampoline(
    vmctx: *mut VMContext,
    callee: *const VMFunctionBody,
    values_vec: *mut u8,
) -> Result<(), String> {
    if WasmtimeCallTrampoline(vmctx as *mut u8, callee, values_vec) == 0 {
        Err(trap_message(vmctx))
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
) -> Result<(), String> {
    if WasmtimeCall(vmctx as *mut u8, callee) == 0 {
        Err(trap_message(vmctx))
    } else {
        Ok(())
    }
}
