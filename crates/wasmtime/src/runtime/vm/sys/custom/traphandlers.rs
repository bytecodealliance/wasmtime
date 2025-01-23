use crate::prelude::*;
use crate::runtime::vm::VMContext;
use core::mem;
use core::ptr::NonNull;

pub use crate::runtime::vm::sys::capi::{self, wasmtime_longjmp};

#[allow(missing_docs)]
pub type SignalHandler = Box<dyn Fn() + Send + Sync>;

pub unsafe fn wasmtime_setjmp(
    jmp_buf: *mut *const u8,
    callback: extern "C" fn(*mut u8, NonNull<VMContext>) -> bool,
    payload: *mut u8,
    callee: NonNull<VMContext>,
) -> bool {
    let callback = mem::transmute::<
        extern "C" fn(*mut u8, NonNull<VMContext>) -> bool,
        extern "C" fn(*mut u8, *mut u8) -> bool,
    >(callback);
    capi::wasmtime_setjmp(jmp_buf, callback, payload, callee.as_ptr().cast())
}

#[cfg(has_native_signals)]
pub struct TrapHandler;

#[cfg(has_native_signals)]
impl TrapHandler {
    pub unsafe fn new(_macos_use_mach_ports: bool) -> TrapHandler {
        capi::wasmtime_init_traps(handle_trap);
        TrapHandler
    }

    pub fn validate_config(&self, _macos_use_mach_ports: bool) {}
}

#[cfg(has_native_signals)]
extern "C" fn handle_trap(pc: usize, fp: usize, has_faulting_addr: bool, faulting_addr: usize) {
    use crate::runtime::vm::traphandlers::{tls, TrapRegisters, TrapTest};

    tls::with(|info| {
        let info = match info {
            Some(info) => info,
            None => return,
        };
        let faulting_addr = if has_faulting_addr {
            Some(faulting_addr)
        } else {
            None
        };
        let regs = TrapRegisters { pc, fp };
        let test = info.test_if_trap(regs, faulting_addr, |_handler| {
            panic!("custom signal handlers are not supported on this platform");
        });
        match test {
            TrapTest::NotWasm => {}
            TrapTest::HandledByEmbedder => unreachable!(),
            TrapTest::Trap { jmp_buf } => unsafe { wasmtime_longjmp(jmp_buf) },
        }
    })
}

pub fn lazy_per_thread_init() {}
