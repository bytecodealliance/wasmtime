//! Support for a calling of an imported function.

use crate::prelude::*;
use crate::runtime::vm::{StoreBox, VMArrayCallHostFuncContext, VMContext, VMOpaqueContext};
use crate::type_registry::RegisteredType;
use crate::{FuncType, ValRaw};
use core::ptr::NonNull;

struct TrampolineState<F> {
    func: F,

    // Need to keep our `VMSharedTypeIndex` registered in the engine.
    #[allow(dead_code)]
    sig: RegisteredType,
}

/// Shim to call a host-defined function that uses the array calling convention.
///
/// Together with `VMArrayCallHostFuncContext`, this implements the transition
/// from a raw, non-closure function pointer to a Rust closure that associates
/// data and function together.
///
/// Also shepherds panics and traps across Wasm.
unsafe extern "C" fn array_call_shim<F>(
    vmctx: NonNull<VMOpaqueContext>,
    caller_vmctx: NonNull<VMOpaqueContext>,
    values_vec: NonNull<ValRaw>,
    values_vec_len: usize,
) -> bool
where
    F: Fn(NonNull<VMContext>, &mut [ValRaw]) -> Result<()> + 'static,
{
    // Be sure to catch Rust panics to manually shepherd them across the wasm
    // boundary, and then otherwise delegate as normal.
    crate::runtime::vm::catch_unwind_and_record_trap(|| {
        let vmctx = VMArrayCallHostFuncContext::from_opaque(vmctx);
        // Double-check ourselves in debug mode, but we control
        // the `Any` here so an unsafe downcast should also
        // work.
        let state = vmctx.as_ref().host_state();
        debug_assert!(state.is::<TrampolineState<F>>());
        let state = &*(state as *const _ as *const TrampolineState<F>);
        let mut values_vec = NonNull::slice_from_raw_parts(values_vec, values_vec_len);
        (state.func)(VMContext::from_opaque(caller_vmctx), values_vec.as_mut())
    })
}

pub fn create_array_call_function<F>(
    ft: &FuncType,
    func: F,
) -> Result<StoreBox<VMArrayCallHostFuncContext>>
where
    F: Fn(NonNull<VMContext>, &mut [ValRaw]) -> Result<()> + Send + Sync + 'static,
{
    let array_call = array_call_shim::<F>;

    let sig = ft.clone().into_registered_type();

    unsafe {
        Ok(VMArrayCallHostFuncContext::new(
            array_call,
            sig.index(),
            Box::new(TrampolineState { func, sig }),
        ))
    }
}
