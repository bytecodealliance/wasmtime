//! Support for a calling of an imported function.

use crate::module::BareModuleInfo;
use crate::{Engine, FuncType, Trap, ValRaw};
use anyhow::Result;
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use wasmtime_environ::{
    AnyfuncIndex, EntityIndex, FunctionInfo, Module, ModuleType, SignatureIndex,
};
use wasmtime_jit::{CodeMemory, ProfilingAgent};
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstanceAllocator, InstanceHandle,
    OnDemandInstanceAllocator, StorePtr, VMContext, VMFunctionBody, VMOpaqueContext,
    VMSharedSignatureIndex, VMTrampoline,
};

struct TrampolineState<F> {
    func: F,
    #[allow(dead_code)]
    code_memory: CodeMemory,
}

unsafe extern "C" fn stub_fn<F>(
    vmctx: *mut VMOpaqueContext,
    caller_vmctx: *mut VMContext,
    values_vec: *mut ValRaw,
    values_vec_len: usize,
) where
    F: Fn(*mut VMContext, &mut [ValRaw]) -> Result<(), Trap> + 'static,
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
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let vmctx = VMContext::from_opaque(vmctx);
        // Double-check ourselves in debug mode, but we control
        // the `Any` here so an unsafe downcast should also
        // work.
        let state = (*vmctx).host_state();
        debug_assert!(state.is::<TrampolineState<F>>());
        let state = &*(state as *const _ as *const TrampolineState<F>);
        let values_vec = std::slice::from_raw_parts_mut(values_vec, values_vec_len);
        (state.func)(caller_vmctx, values_vec)
    }));

    match result {
        Ok(Ok(())) => {}

        // If a trap was raised (an error returned from the imported function)
        // then we smuggle the trap through `Box<dyn Error>` through to the
        // call-site, which gets unwrapped in `Trap::from_runtime` later on as we
        // convert from the internal `Trap` type to our own `Trap` type in this
        // crate.
        Ok(Err(trap)) => wasmtime_runtime::raise_user_trap(trap.into()),

        // And finally if the imported function panicked, then we trigger the
        // form of unwinding that's safe to jump over wasm code on all
        // platforms.
        Err(panic) => wasmtime_runtime::resume_panic(panic),
    }
}

#[cfg(compiler)]
fn register_trampolines(profiler: &dyn ProfilingAgent, image: &object::File<'_>) {
    use object::{Object as _, ObjectSection, ObjectSymbol, SectionKind, SymbolKind};
    let pid = std::process::id();
    let tid = pid;

    let text_base = match image.sections().find(|s| s.kind() == SectionKind::Text) {
        Some(section) => match section.data() {
            Ok(data) => data.as_ptr() as usize,
            Err(_) => return,
        },
        None => return,
    };

    for sym in image.symbols() {
        if !sym.is_definition() {
            continue;
        }
        if sym.kind() != SymbolKind::Text {
            continue;
        }
        let address = sym.address();
        let size = sym.size();
        if address == 0 || size == 0 {
            continue;
        }
        if let Ok(name) = sym.name() {
            let addr = text_base + address as usize;
            profiler.load_single_trampoline(name, addr as *const u8, size as usize, pid, tid);
        }
    }
}

#[cfg(compiler)]
pub fn create_function<F>(
    ft: &FuncType,
    func: F,
    engine: &Engine,
) -> Result<(InstanceHandle, VMTrampoline)>
where
    F: Fn(*mut VMContext, &mut [ValRaw]) -> Result<(), Trap> + Send + Sync + 'static,
{
    let mut obj = engine.compiler().object()?;
    let (t1, t2) = engine.compiler().emit_trampoline_obj(
        ft.as_wasm_func_type(),
        stub_fn::<F> as usize,
        &mut obj,
    )?;
    let obj = wasmtime_jit::mmap_vec_from_obj(obj)?;

    // Copy the results of JIT compilation into executable memory, and this will
    // also take care of unwind table registration.
    let mut code_memory = CodeMemory::new(obj);
    let code = code_memory.publish()?;

    register_trampolines(engine.config().profiler.as_ref(), &code.obj);

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

    let sig_id = SignatureIndex::from_u32(u32::max_value() - 1);
    module.types.push(ModuleType::Function(sig_id));
    let func_id = module.push_escaped_function(sig_id, AnyfuncIndex::from_u32(0));
    module.num_escaped_funcs = 1;
    module
        .exports
        .insert(String::new(), EntityIndex::Function(func_id));
    let module = Arc::new(module);

    let runtime_info = &BareModuleInfo::one_func(
        module.clone(),
        (*func).as_ptr() as usize,
        FunctionInfo::default(),
        sig_id,
        sig,
    )
    .into_traitobj();

    Ok(
        OnDemandInstanceAllocator::default().allocate(InstanceAllocationRequest {
            imports: Imports::default(),
            host_state,
            store: StorePtr::empty(),
            runtime_info,
        })?,
    )
}
