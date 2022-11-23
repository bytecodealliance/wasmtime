//! Trampolines for calling into Wasm from the host and calling the host from
//! Wasm.

use crate::VMContext;
use std::mem;

/// Given a Wasm function pointer and a `vmctx`, prepare the `vmctx` for calling
/// into that Wasm function, and return the host-to-Wasm entry trampoline.
///
/// Callers must never call Wasm function pointers directly. Callers must
/// instead call this function and then enter Wasm through the returned
/// host-to-Wasm trampoline.
///
/// # Unsafety
///
/// The `vmctx` argument must be valid.
///
/// The generic type `T` must be a function pointer type and `func` must be a
/// pointer to a Wasm function of that signature.
///
/// After calling this function, you may not mess with the vmctx or any other
/// Wasm state until after you've called the trampoline returned by this
/// function.
#[inline]
pub unsafe fn prepare_host_to_wasm_trampoline<T>(vmctx: *mut VMContext, func: T) -> T {
    assert_eq!(mem::size_of::<T>(), mem::size_of::<usize>());

    // Save the callee in the `vmctx`. The trampoline will read this function
    // pointer and tail call to it.
    (*vmctx)
        .instance_mut()
        .set_callee(Some(mem::transmute_copy(&func)));

    // Give callers the trampoline, transmuted into their desired function
    // signature (the trampoline is variadic and works with all signatures).
    mem::transmute_copy(&(host_to_wasm_trampoline as usize))
}

extern "C" {
    fn host_to_wasm_trampoline();
    pub(crate) fn wasm_to_host_trampoline();
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        #[macro_use]
        mod x86_64;
    } else if #[cfg(target_arch = "aarch64")] {
        #[macro_use]
        mod aarch64;
    } else if #[cfg(target_arch = "s390x")] {
        #[macro_use]
        mod s390x;
    }else if #[cfg(target_arch = "riscv64")] {
        #[macro_use]
        mod riscv64;
    } else {
        compile_error!("unsupported architecture");
    }
}
