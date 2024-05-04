//! Support for a calling of an imported function.

use crate::runtime::vm::{
    StoreBox, VMArrayCallHostFuncContext, VMContext, VMFuncRef, VMOpaqueContext,
};
use crate::type_registry::RegisteredType;
use crate::{code_memory::CodeMemory, Engine, FuncType, ValRaw};
use anyhow::Result;
use core::ptr::NonNull;

struct TrampolineState<F> {
    func: F,
    #[allow(dead_code)]
    code_memory: CodeMemory,

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
    vmctx: *mut VMOpaqueContext,
    caller_vmctx: *mut VMOpaqueContext,
    values_vec: *mut ValRaw,
    values_vec_len: usize,
) where
    F: Fn(*mut VMContext, &mut [ValRaw]) -> Result<()> + 'static,
{
    // Here we are careful to use `catch_unwind` to ensure Rust panics don't
    // unwind past us. The primary reason for this is that Rust considers it UB
    // to unwind past an `extern "C"` function. Here we are in an `extern "C"`
    // function and the cross into wasm was through an `extern "C"` function at
    // the base of the stack as well. We'll need to wait for assorted RFCs and
    // language features to enable this to be done in a sound and stable fashion
    // before avoiding catching the panic here.
    //
    // Also note that there are intentionally no local variables on this stack
    // frame. The reason for that is that some of the "raise" functions we have
    // below will trigger a longjmp, which won't run local destructors if we
    // have any. To prevent leaks we avoid having any local destructors by
    // avoiding local variables.
    let result = crate::runtime::vm::catch_unwind_and_longjmp(|| {
        let vmctx = VMArrayCallHostFuncContext::from_opaque(vmctx);
        // Double-check ourselves in debug mode, but we control
        // the `Any` here so an unsafe downcast should also
        // work.
        let state = (*vmctx).host_state();
        debug_assert!(state.is::<TrampolineState<F>>());
        let state = &*(state as *const _ as *const TrampolineState<F>);
        let values_vec = core::slice::from_raw_parts_mut(values_vec, values_vec_len);
        (state.func)(VMContext::from_opaque(caller_vmctx), values_vec)
    });

    match result {
        Ok(()) => {}

        // If a trap was raised (an error returned from the imported function)
        // then we smuggle the trap through `Box<dyn Error>` through to the
        // call-site, which gets unwrapped in `Trap::from_runtime` later on as we
        // convert from the internal `Trap` type to our own `Trap` type in this
        // crate.
        Err(trap) => crate::trap::raise(trap.into()),
    }
}

#[cfg(any(feature = "cranelift", feature = "winch"))]
pub fn create_array_call_function<F>(
    ft: &FuncType,
    func: F,
    engine: &Engine,
) -> Result<StoreBox<VMArrayCallHostFuncContext>>
where
    F: Fn(*mut VMContext, &mut [ValRaw]) -> Result<()> + Send + Sync + 'static,
{
    use crate::compile::finish_object;
    use crate::prelude::*;
    use std::ptr;

    let mut obj = engine
        .compiler()
        .object(wasmtime_environ::ObjectKind::Module)?;
    let (wasm_call_range, native_call_range) = engine
        .compiler()
        .emit_trampolines_for_array_call_host_func(
            ft.as_wasm_func_type(),
            array_call_shim::<F> as usize,
            &mut obj,
        )?;
    engine.append_bti(&mut obj);
    let obj = finish_object(wasmtime_environ::ObjectBuilder::new(obj, engine.tunables()))?;

    // Copy the results of JIT compilation into executable memory, and this will
    // also take care of unwind table registration.
    let mut code_memory = CodeMemory::new(obj)?;
    code_memory.publish()?;

    engine
        .profiler()
        .register_module(&code_memory.mmap()[..], &|_| None);

    // Extract the host/wasm trampolines from the results of compilation since
    // we know their start/length.

    let text = code_memory.text();

    let array_call = array_call_shim::<F>;

    let wasm_call = text[wasm_call_range.start as usize..].as_ptr() as *mut _;
    let wasm_call = Some(NonNull::new(wasm_call).unwrap());

    let native_call = text[native_call_range.start as usize..].as_ptr() as *mut _;
    let native_call = NonNull::new(native_call).unwrap();

    let sig = ft.clone().into_registered_type();

    unsafe {
        Ok(VMArrayCallHostFuncContext::new(
            VMFuncRef {
                array_call,
                wasm_call,
                native_call,
                type_index: sig.index(),
                vmctx: ptr::null_mut(),
            },
            Box::new(TrampolineState {
                func,
                code_memory,
                sig,
            }),
        ))
    }
}
