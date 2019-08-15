//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use crate::vmcontext::{VMContext, VMFunctionBody};
use core::cell::{Cell, RefCell};
use core::ffi::c_void;
use core::mem;
use core::ptr;
use std::string::String;
use std::vec::Vec;
#[cfg(target_os = "windows")]
use winapi::ctypes::c_int;

extern "C" {
    fn WasmtimeSjljCallTrampoline(
        buf: *mut c_void,
        vmctx: *mut c_void,
        callee: *const c_void,
        args: *mut c_void,
    ) -> libc::c_int;
    fn WasmtimeSjljCall(buf: *mut c_void, vmctx: *mut c_void, callee: *const c_void)
        -> libc::c_int;
    fn SjljUnwind(buf: *mut i8);
    #[cfg(target_os = "windows")]
    fn _resetstkoflw() -> c_int;
}

const LLVM_SJLJ_BUF_SIZE: usize = 5 * mem::size_of::<usize>();
#[repr(C, align(16))]
#[derive(Clone)]
pub struct JmpBuf([i8; LLVM_SJLJ_BUF_SIZE]);

thread_local! {
    static FIX_STACK: Cell<bool> = Cell::new(false);
    static TRAP_PC: Cell<*const u8> = Cell::new(ptr::null());
    static JMP_BUFS: RefCell<Vec<JmpBuf>> = RefCell::new(Vec::new());
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
        let mut buf = bufs.borrow_mut().pop().unwrap();
        unsafe { SjljUnwind(buf.0.as_mut_ptr()) };
    })
}

/// Schedules fixing the stack after unwinding
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn FixStackAfterUnwinding() {
    FIX_STACK.with(|fix_stack| {
        assert_eq!(fix_stack.get(), false);
        fix_stack.set(true)
    });
}

/// A simple guard to ensure that `JMP_BUFS` is reset when we're done.
struct ScopeGuard {
    orig_num_bufs: usize,
}

impl ScopeGuard {
    fn new() -> Self {
        assert_eq!(
            TRAP_PC.with(Cell::get),
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

/// Used by wasmtime call trampolines to save jmp_buf.
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn PushJmpBuffer(buf: *mut JmpBuf) {
    JMP_BUFS.with(|bufs| bufs.borrow_mut().push((*buf).clone()));
}

fn run_post_unwind_actions() {
    FIX_STACK.with(|fix_stack| {
        if fix_stack.get() {
            #[cfg(target_os = "windows")]
            {
                // We need to restore guard page under stack to handle future stack overflows properly.
                // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/resetstkoflw?view=vs-2019
                if unsafe { _resetstkoflw() } == 0 {
                    panic!("Failed to fix the stack after unwinding");
                }
            }
            fix_stack.set(false);
        }
    })
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
    // Reset JMP_BUFS if the stack is unwound through this point.
    let _guard = ScopeGuard::new();

    let mut jmp_buf = mem::uninitialized::<JmpBuf>();

    if WasmtimeSjljCallTrampoline(
        jmp_buf.0.as_mut_ptr() as *mut _,
        vmctx as *mut _,
        callee as *const _,
        values_vec as *mut _,
    ) == 0
    {
        Ok(())
    } else {
        run_post_unwind_actions();
        Err(trap_message(vmctx))
    }
}

/// Call the wasm function pointed to by `callee`, which has no arguments or
/// return values.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call(
    vmctx: *mut VMContext,
    callee: *const VMFunctionBody,
) -> Result<(), String> {
    // Reset JMP_BUFS if the stack is unwound through this point.
    let _guard = ScopeGuard::new();

    let mut jmp_buf = mem::uninitialized::<JmpBuf>();

    if WasmtimeSjljCall(
        jmp_buf.0.as_mut_ptr() as *mut _,
        vmctx as *mut _,
        callee as *const _,
    ) == 0
    {
        Ok(())
    } else {
        run_post_unwind_actions();
        Err(trap_message(vmctx))
    }
}
