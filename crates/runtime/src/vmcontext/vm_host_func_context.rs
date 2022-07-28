//! Definition of `VM*Context` variant for host functions.
//!
//! Keep in sync with `wasmtime_environ::VMHostFuncOffsets`.

use wasmtime_environ::VM_HOST_FUNC_MAGIC;

use super::{VMCallerCheckedAnyfunc, VMFunctionBody, VMOpaqueContext, VMSharedSignatureIndex};
use std::{
    any::Any,
    ptr::{self, NonNull},
};

/// The `VM*Context` for host functions.
///
/// Its `magic` field must always be `wasmtime_environ::VM_HOST_FUNC_MAGIC`, and
/// this is how you can determine whether a `VM*Context` is a
/// `VMHostFuncContext` versus a different kind of context.
#[repr(C)]
pub struct VMHostFuncContext {
    magic: u32,
    // _padding: u32, // (on 64-bit systems)
    pub(crate) host_func: NonNull<VMFunctionBody>,
    wasm_to_host_trampoline: VMCallerCheckedAnyfunc,
    host_state: Box<dyn Any + Send + Sync>,
}

// Declare that this type is send/sync, it's the responsibility of
// `VMHostFuncContext::new` callers to uphold this guarantee.
unsafe impl Send for VMHostFuncContext {}
unsafe impl Sync for VMHostFuncContext {}

impl VMHostFuncContext {
    /// Create the context for the given host function.
    ///
    /// # Safety
    ///
    /// The `host_func` must be a pointer to a host (not Wasm) function and it
    /// must be `Send` and `Sync`.
    pub unsafe fn new(
        host_func: NonNull<VMFunctionBody>,
        signature: VMSharedSignatureIndex,
        host_state: Box<dyn Any + Send + Sync>,
    ) -> Box<VMHostFuncContext> {
        let wasm_to_host_trampoline = VMCallerCheckedAnyfunc {
            func_ptr: NonNull::new(crate::trampolines::wasm_to_host_trampoline as _).unwrap(),
            type_index: signature,
            vmctx: ptr::null_mut(),
        };
        let mut ctx = Box::new(VMHostFuncContext {
            magic: wasmtime_environ::VM_HOST_FUNC_MAGIC,
            host_func,
            wasm_to_host_trampoline,
            host_state,
        });
        ctx.wasm_to_host_trampoline.vmctx =
            VMOpaqueContext::from_vm_host_func_context(&*ctx as *const _ as *mut _);
        ctx
    }

    /// Get the Wasm-to-host trampoline for this host function context.
    pub fn wasm_to_host_trampoline(&self) -> NonNull<VMCallerCheckedAnyfunc> {
        NonNull::from(&self.wasm_to_host_trampoline)
    }

    /// Get the host state for this host function context.
    pub fn host_state(&self) -> &(dyn Any + Send + Sync) {
        &*self.host_state
    }
}

impl VMHostFuncContext {
    /// Helper function to cast between context types using a debug assertion to
    /// protect against some mistakes.
    #[inline]
    pub unsafe fn from_opaque(opaque: *mut VMOpaqueContext) -> *mut VMHostFuncContext {
        // See comments in `VMContext::from_opaque` for this debug assert
        debug_assert_eq!((*opaque).magic, VM_HOST_FUNC_MAGIC);
        opaque.cast()
    }
}
