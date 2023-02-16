//! Support for a calling of an imported function.

use crate::{Engine, FuncType, ValRaw};
use anyhow::Result;
use std::panic::{self, AssertUnwindSafe};
use std::ptr::NonNull;
use wasmtime_jit::{CodeMemory, ProfilingAgent};
use wasmtime_runtime::{
    VMContext, VMHostFuncContext, VMOpaqueContext, VMSharedSignatureIndex, VMTrampoline,
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
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let vmctx = VMHostFuncContext::from_opaque(vmctx);
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
        Ok(Err(trap)) => crate::trap::raise(trap.into()),

        // And finally if the imported function panicked, then we trigger the
        // form of unwinding that's safe to jump over wasm code on all
        // platforms.
        Err(panic) => wasmtime_runtime::resume_panic(panic),
    }
}

#[cfg(compiler)]
fn register_trampolines(profiler: &dyn ProfilingAgent, code: &CodeMemory) {
    use object::{File, Object as _, ObjectSection, ObjectSymbol, SectionKind, SymbolKind};
    let pid = std::process::id();
    let tid = pid;

    let image = match File::parse(&code.mmap()[..]) {
        Ok(image) => image,
        Err(_) => return,
    };

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
) -> Result<(Box<VMHostFuncContext>, VMSharedSignatureIndex, VMTrampoline)>
where
    F: Fn(*mut VMContext, &mut [ValRaw]) -> Result<()> + Send + Sync + 'static,
{
    let mut obj = engine
        .compiler()
        .object(wasmtime_environ::ObjectKind::Module)?;
    let (t1, t2) = engine.compiler().emit_trampoline_obj(
        ft.as_wasm_func_type(),
        stub_fn::<F> as usize,
        &mut obj,
    )?;
    engine.append_bti(&mut obj);
    let obj = wasmtime_jit::ObjectBuilder::new(obj, &engine.config().tunables).finish()?;

    // Copy the results of JIT compilation into executable memory, and this will
    // also take care of unwind table registration.
    let mut code_memory = CodeMemory::new(obj)?;
    code_memory.publish()?;

    register_trampolines(engine.profiler(), &code_memory);

    // Extract the host/wasm trampolines from the results of compilation since
    // we know their start/length.

    let text = code_memory.text();
    let host_trampoline = text[t1.start as usize..][..t1.length as usize].as_ptr();
    let wasm_trampoline = text[t2.start as usize..].as_ptr() as *mut _;
    let wasm_trampoline = NonNull::new(wasm_trampoline).unwrap();

    let sig = engine.signatures().register(ft.as_wasm_func_type());

    unsafe {
        let ctx = VMHostFuncContext::new(
            wasm_trampoline,
            sig,
            Box::new(TrampolineState { func, code_memory }),
        );
        let host_trampoline = std::mem::transmute::<*const u8, VMTrampoline>(host_trampoline);
        Ok((ctx, sig, host_trampoline))
    }
}
