//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::signalhandlers::jmp_buf;
use crate::vmcontext::{VMContext, VMFunctionBody};
use core::cell::{Cell, RefCell};
use core::mem;
use core::ptr;
use libc::c_int;
use std::string::String;
use std::vec::Vec;

// Currently we uset setjmp/longjmp to unwind out of a signal handler
// and back to the point where WebAssembly was called (via `call_wasm`).
// This works because WebAssembly code currently does not use any EH
// or require any cleanups, and we never unwind through non-wasm frames.
// In the future, we'll likely replace this with fancier stack unwinding.
extern "C" {
    fn setjmp(env: *mut jmp_buf) -> c_int;
    fn longjmp(env: *const jmp_buf, val: c_int) -> !;
}

thread_local! {
    static TRAP_PC: Cell<*const u8> = Cell::new(ptr::null());
    static JMP_BUFS: RefCell<Vec<jmp_buf>> = RefCell::new(Vec::new());
}

/// Record the Trap code and wasm bytecode offset in TLS somewhere
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RecordTrap(pc: *const u8) {
    // TODO: Look up the wasm bytecode offset and trap code and record them instead.
    TRAP_PC.with(|data| data.set(pc));
}

/// Initiate an unwind.
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn Unwind() {
    JMP_BUFS.with(|bufs| {
        let buf = bufs.borrow_mut().pop().unwrap();
        unsafe { longjmp(&buf, 1) };
    })
}

/// A simple guard to ensure that `JMP_BUFS` is reset when we're done.
struct ScopeGuard {
    orig_num_bufs: usize,
}

impl ScopeGuard {
    fn new() -> Self {
        assert_eq!(
            TRAP_PC.with(|data| data.get()),
            ptr::null(),
            "unfinished trap detected"
        );
        Self {
            orig_num_bufs: JMP_BUFS.with(|bufs| bufs.borrow().len()),
        }
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let orig_num_bufs = self.orig_num_bufs;
        JMP_BUFS.with(|bufs| {
            bufs.borrow_mut()
                .resize(orig_num_bufs, unsafe { mem::zeroed() })
        });
    }
}

fn trap_message(_vmctx: *mut VMContext) -> String {
    let pc = TRAP_PC.with(|data| data.replace(ptr::null()));

    // TODO: Record trap metadata in the VMContext, and look up the
    // pc to obtain the TrapCode and SourceLoc.

    format!("wasm trap at {:?}", pc)
}

fn push_jmp_buf(buf: jmp_buf) {
    JMP_BUFS.with(|bufs| bufs.borrow_mut().push(buf));
}

/// Call the wasm function pointed to by `callee`. `values_vec` points to
/// a buffer which holds the incoming arguments, and to which the outgoing
/// return values will be written.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call_trampoline(
    callee: *const VMFunctionBody,
    values_vec: *mut u8,
    vmctx: *mut VMContext,
) -> Result<(), String> {
    // Reset JMP_BUFS if the stack is unwound through this point.
    let _guard = ScopeGuard::new();

    // Set a setjmp catch point.
    let mut buf = mem::uninitialized();
    if setjmp(&mut buf) != 0 {
        return Err(trap_message(vmctx));
    }
    push_jmp_buf(buf);

    // Call the function!
    let func: fn(*mut u8, *mut VMContext) = mem::transmute(callee);
    func(values_vec, vmctx);

    Ok(())
}

/// Call the wasm function pointed to by `callee`, which has no arguments or
/// return values.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call(
    callee: *const VMFunctionBody,
    vmctx: *mut VMContext,
) -> Result<(), String> {
    // Reset JMP_BUFS if the stack is unwound through this point.
    let _guard = ScopeGuard::new();

    // Set a setjmp catch point.
    let mut buf = mem::uninitialized();
    if setjmp(&mut buf) != 0 {
        return Err(trap_message(vmctx));
    }
    push_jmp_buf(buf);

    // Call the function!
    let func: fn(*mut VMContext) = mem::transmute(callee);
    func(vmctx);

    Ok(())
}
