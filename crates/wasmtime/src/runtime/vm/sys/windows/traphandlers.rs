use crate::runtime::vm::traphandlers::{tls, TrapTest};
use crate::runtime::vm::VMContext;
use std::io;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::System::Kernel::*;

#[link(name = "wasmtime-helpers")]
extern "C" {
    #[wasmtime_versioned_export_macros::versioned_link]
    #[allow(improper_ctypes)]
    pub fn wasmtime_setjmp(
        jmp_buf: *mut *const u8,
        callback: extern "C" fn(*mut u8, *mut VMContext),
        payload: *mut u8,
        callee: *mut VMContext,
    ) -> i32;

    #[wasmtime_versioned_export_macros::versioned_link]
    pub fn wasmtime_longjmp(jmp_buf: *const u8) -> !;
}

/// Function which may handle custom signals while processing traps.
pub type SignalHandler<'a> = dyn Fn(*mut EXCEPTION_POINTERS) -> bool + Send + Sync + 'a;

pub unsafe fn platform_init(_macos_use_mach_ports: bool) {
    // our trap handler needs to go first, so that we can recover from
    // wasm faults and continue execution, so pass `1` as a true value
    // here.
    if AddVectoredExceptionHandler(1, Some(exception_handler)).is_null() {
        panic!(
            "failed to add exception handler: {}",
            io::Error::last_os_error()
        );
    }
}

unsafe extern "system" fn exception_handler(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    // Check the kind of exception, since we only handle a subset within
    // wasm code. If anything else happens we want to defer to whatever
    // the rest of the system wants to do for this exception.
    let record = &*(*exception_info).ExceptionRecord;
    if record.ExceptionCode != EXCEPTION_ACCESS_VIOLATION
        && record.ExceptionCode != EXCEPTION_ILLEGAL_INSTRUCTION
        && record.ExceptionCode != EXCEPTION_INT_DIVIDE_BY_ZERO
        && record.ExceptionCode != EXCEPTION_INT_OVERFLOW
    {
        return ExceptionContinueSearch;
    }

    // FIXME: this is what the previous C++ did to make sure that TLS
    // works by the time we execute this trap handling code. This isn't
    // exactly super easy to call from Rust though and it's not clear we
    // necessarily need to do so. Leaving this here in case we need this
    // in the future, but for now we can probably wait until we see a
    // strange fault before figuring out how to reimplement this in
    // Rust.
    //
    // if (!NtCurrentTeb()->Reserved1[sThreadLocalArrayPointerIndex]) {
    //     return ExceptionContinueSearch;
    // }

    // This is basically the same as the unix version above, only with a
    // few parameters tweaked here and there.
    tls::with(|info| {
        let info = match info {
            Some(info) => info,
            None => return ExceptionContinueSearch,
        };
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                let ip = (*(*exception_info).ContextRecord).Rip as *const u8;
                let fp = (*(*exception_info).ContextRecord).Rbp as usize;
            } else if #[cfg(target_arch = "aarch64")] {
                let ip = (*(*exception_info).ContextRecord).Pc as *const u8;
                let fp = (*(*exception_info).ContextRecord).Anonymous.Anonymous.Fp as usize;
            } else {
                compile_error!("unsupported platform");
            }
        }
        // For access violations the first element in `ExceptionInformation` is
        // an indicator as to whether the fault was a read/write. The second
        // element is the address of the inaccessible data causing this
        // violation.
        let faulting_addr = if record.ExceptionCode == EXCEPTION_ACCESS_VIOLATION {
            assert!(record.NumberParameters >= 2);
            Some(record.ExceptionInformation[1])
        } else {
            None
        };
        match info.test_if_trap(ip, |handler| handler(exception_info)) {
            TrapTest::NotWasm => ExceptionContinueSearch,
            TrapTest::HandledByEmbedder => ExceptionContinueExecution,
            TrapTest::Trap { jmp_buf, trap } => {
                info.set_jit_trap(ip, fp, faulting_addr, trap);
                wasmtime_longjmp(jmp_buf)
            }
        }
    })
}

pub fn lazy_per_thread_init() {
    // Unused on Windows
}
