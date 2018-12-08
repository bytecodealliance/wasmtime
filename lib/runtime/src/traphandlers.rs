//! WebAssembly trap handling, which is built on top of the lower-level
//! signalhandling mechanisms.

use libc::c_int;
use signalhandlers::{jmp_buf, CodeSegment};
use std::cell::{Cell, RefCell};
use std::mem;
use std::ptr;
use std::string::String;
use vmcontext::{VMContext, VMFunctionBody};

// Currently we uset setjmp/longjmp to unwind out of a signal handler
// and back to the point where WebAssembly was called (via `call_wasm`).
// This works because WebAssembly code currently does not use any EH
// or require any cleanups, and we never unwind through non-wasm frames.
// In the future, we'll likely replace this with fancier stack unwinding.
extern "C" {
    fn setjmp(env: *mut jmp_buf) -> c_int;
    fn longjmp(env: *const jmp_buf, val: c_int) -> !;
}

#[derive(Copy, Clone, Debug)]
struct TrapData {
    pc: *const u8,
}

thread_local! {
    static TRAP_DATA: Cell<TrapData> = Cell::new(TrapData { pc: ptr::null() });
    static JMP_BUFS: RefCell<Vec<jmp_buf>> = RefCell::new(Vec::new());
}

/// Record the Trap code and wasm bytecode offset in TLS somewhere
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RecordTrap(pc: *const u8, _codeSegment: *const CodeSegment) {
    // TODO: Look up the wasm bytecode offset and trap code and record them instead.
    TRAP_DATA.with(|data| data.set(TrapData { pc }));
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

/// Return the CodeSegment containing the given pc, if any exist in the process.
/// This method does not take a lock.
#[doc(hidden)]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn LookupCodeSegment(_pc: *const ::std::os::raw::c_void) -> *const CodeSegment {
    // TODO: Implement this.
    -1isize as *const CodeSegment
}

/// A simple guard to ensure that `JMP_BUFS` is reset when we're done.
struct ScopeGuard {
    orig_num_bufs: usize,
}

impl ScopeGuard {
    fn new() -> Self {
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

/// Call the wasm function pointed to by `callee`. `values_vec` points to
/// a buffer which holds the incoming arguments, and to which the outgoing
/// return values will be written.
#[no_mangle]
pub unsafe extern "C" fn wasmtime_call_trampoline(
    callee: *const VMFunctionBody,
    values_vec: *mut u8,
    vmctx: *mut VMContext,
) -> Result<(), String> {
    // In case wasm code calls Rust that panics and unwinds past this point,
    // ensure that JMP_BUFS is unwound to its incoming state.
    let _guard = ScopeGuard::new();

    let func: fn(*mut u8, *mut VMContext) = mem::transmute(callee);

    JMP_BUFS.with(|bufs| {
        let mut buf = mem::uninitialized();
        if setjmp(&mut buf) != 0 {
            return TRAP_DATA.with(|data| Err(format!("wasm trap at {:?}", data.get().pc)));
        }
        bufs.borrow_mut().push(buf);
        func(values_vec, vmctx);
        Ok(())
    })
}
