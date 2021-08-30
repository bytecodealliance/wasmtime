//! Support for a calling of an imported function.

use crate::{Engine, FuncType, Trap};
use anyhow::Result;
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use wasmtime_environ::{EntityIndex, Module, ModuleType, PrimaryMap, SignatureIndex};
use wasmtime_jit::{CodeMemory, MmapVec};
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstanceAllocator, InstanceHandle,
    OnDemandInstanceAllocator, VMContext, VMFunctionBody, VMSharedSignatureIndex, VMTrampoline,
};

struct TrampolineState {
    func: Box<dyn Fn(*mut VMContext, *mut u128) -> Result<(), Trap> + Send + Sync>,
    #[allow(dead_code)]
    code_memory: CodeMemory,
}

unsafe extern "C" fn stub_fn(
    vmctx: *mut VMContext,
    caller_vmctx: *mut VMContext,
    values_vec: *mut u128,
) {
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
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        call_stub(vmctx, caller_vmctx, values_vec)
    }));

    match result {
        Ok(Ok(())) => {}

        // If a trap was raised (an error returned from the imported function)
        // then we smuggle the trap through `Box<dyn Error>` through to the
        // call-site, which gets unwrapped in `Trap::from_runtime` later on as we
        // convert from the internal `Trap` type to our own `Trap` type in this
        // crate.
        Ok(Err(trap)) => wasmtime_runtime::raise_user_trap(Box::new(trap)),

        // And finally if the imported function panicked, then we trigger the
        // form of unwinding that's safe to jump over wasm code on all
        // platforms.
        Err(panic) => wasmtime_runtime::resume_panic(panic),
    }

    unsafe fn call_stub(
        vmctx: *mut VMContext,
        caller_vmctx: *mut VMContext,
        values_vec: *mut u128,
    ) -> Result<(), Trap> {
        let instance = InstanceHandle::from_vmctx(vmctx);
        let state = &instance
            .host_state()
            .downcast_ref::<TrampolineState>()
            .expect("state");
        (state.func)(caller_vmctx, values_vec)
    }
}

#[cfg(compiler)]
pub fn create_function(
    ft: &FuncType,
    func: Box<dyn Fn(*mut VMContext, *mut u128) -> Result<(), Trap> + Send + Sync>,
    engine: &Engine,
) -> Result<(InstanceHandle, VMTrampoline)> {
    let mut obj = engine.compiler().object()?;
    let (t1, t2) = engine.compiler().emit_trampoline_obj(
        ft.as_wasm_func_type(),
        stub_fn as usize,
        &mut obj,
    )?;
    let obj = MmapVec::from_obj(obj)?;

    // Copy the results of JIT compilation into executable memory, and this will
    // also take care of unwind table registration.
    let mut code_memory = CodeMemory::new(obj);
    let code = code_memory.publish()?;

    // Extract the host/wasm trampolines from the results of compilation since
    // we know their start/length.
    let host_trampoline = code.text[t1.start as usize..][..t1.length as usize].as_ptr();
    let wasm_trampoline = &code.text[t2.start as usize..][..t2.length as usize];
    let wasm_trampoline = wasm_trampoline as *const [u8] as *mut [VMFunctionBody];

    let sig = engine.signatures().register(ft.as_wasm_func_type());

    unsafe {
        let instance = create_raw_function(
            wasm_trampoline,
            sig,
            Box::new(TrampolineState { func, code_memory }),
        )?;
        let host_trampoline = std::mem::transmute::<*const u8, VMTrampoline>(host_trampoline);
        Ok((instance, host_trampoline))
    }
}

pub unsafe fn create_raw_function(
    func: *mut [VMFunctionBody],
    sig: VMSharedSignatureIndex,
    host_state: Box<dyn Any + Send + Sync>,
) -> Result<InstanceHandle> {
    let mut module = Module::new();
    let mut functions = PrimaryMap::new();
    functions.push(Default::default());

    let sig_id = SignatureIndex::from_u32(u32::max_value() - 1);
    module.types.push(ModuleType::Function(sig_id));
    let func_id = module.functions.push(sig_id);
    module
        .exports
        .insert(String::new(), EntityIndex::Function(func_id));

    Ok(
        OnDemandInstanceAllocator::default().allocate(InstanceAllocationRequest {
            module: Arc::new(module),
            functions: &functions,
            image_base: (*func).as_ptr() as usize,
            imports: Imports::default(),
            shared_signatures: sig.into(),
            host_state,
            store: None,
            wasm_data: &[],
        })?,
    )
}
