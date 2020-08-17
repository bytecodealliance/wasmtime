#![allow(missing_docs)]

use crate::runtime::{Engine, EngineInner};
use std::sync::{Arc, Mutex, Weak};
use wasmtime_jit::CompiledModule;

pub use wasmtime_runtime::debugger::{
    BreakpointData, DebuggerContext, DebuggerContextData, DebuggerPauseKind, DebuggerResumeAction,
    PatchableCode,
};

pub trait DebuggerJitCodeRegistration: std::marker::Send + std::marker::Sync {
    fn id(&self) -> u32;
}

pub trait DebuggerAgent: std::marker::Send + std::marker::Sync {
    fn pause(&mut self, kind: DebuggerPauseKind) -> DebuggerResumeAction;
    fn register_module(&mut self, module: DebuggerModule) -> Box<dyn DebuggerJitCodeRegistration>;
    fn add_breakpoints(&self, module_id: u32, addr: u64);
    fn find_breakpoint(&self, addr: usize) -> Option<*const BreakpointData>;
}

pub struct DebuggerModule<'a> {
    module: Weak<CompiledModule>,
    engine: Weak<EngineInner>,
    bytes: &'a [u8],
}

impl<'a> DebuggerModule<'a> {
    pub(crate) fn new(
        module: &Arc<CompiledModule>,
        engine: Weak<EngineInner>,
        bytes: &'a [u8],
    ) -> Self {
        Self {
            module: Arc::downgrade(module),
            engine,
            bytes,
        }
    }
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }
    pub fn compiled_module(&self) -> Weak<CompiledModule> {
        self.module.clone()
    }
    pub fn engine(&self) -> Engine {
        self.engine.upgrade().unwrap().into()
    }
    fn module(&self) -> Arc<CompiledModule> {
        self.module.upgrade().unwrap()
    }
    pub fn ranges(&self) -> Vec<(usize, usize)> {
        self.module().jit_code_ranges().collect()
    }
    pub fn name(&self) -> Option<String> {
        self.module().module().name.clone()
    }
}

pub(crate) struct NullDebuggerAgent;

impl DebuggerAgent for NullDebuggerAgent {
    fn pause(&mut self, _kind: DebuggerPauseKind) -> DebuggerResumeAction {
        DebuggerResumeAction::Continue
    }
    fn register_module(&mut self, _module: DebuggerModule) -> Box<dyn DebuggerJitCodeRegistration> {
        struct NullReg;
        impl DebuggerJitCodeRegistration for NullReg {
            fn id(&self) -> u32 {
                0
            }
        }
        Box::new(NullReg)
    }
    fn find_breakpoint(&self, _addr: usize) -> Option<*const BreakpointData> {
        None
    }
    fn add_breakpoints(&self, _module_id: u32, _addr: u64) {
        panic!()
    }
}

pub(crate) struct EngineDebuggerContext {
    engine: Weak<EngineInner>,
    data: Mutex<Option<Box<dyn std::any::Any + Send + Sync>>>,
}

fn _assert_engine_debugger_context_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<EngineDebuggerContext>();
}

impl EngineDebuggerContext {
    pub fn new(engine: &Engine) -> EngineDebuggerContext {
        EngineDebuggerContext::new_inner(engine.clone().weak())
    }
    pub(crate) fn new_inner(engine: Weak<EngineInner>) -> EngineDebuggerContext {
        EngineDebuggerContext {
            engine,
            data: Mutex::new(None),
        }
    }
    fn debugger(&self) -> Arc<Mutex<dyn DebuggerAgent + 'static>> {
        let engine_inner = self.engine.upgrade().unwrap();
        engine_inner.config().debugger.clone()
    }
}

impl DebuggerContext for EngineDebuggerContext {
    fn patchable(&self) -> &dyn PatchableCode {
        self
    }
    fn find_breakpoint(&self, addr: *const u8) -> Option<*const BreakpointData> {
        let addr = addr as usize;
        let engine_inner = self.engine.upgrade().unwrap();
        engine_inner.find_breakpoint(addr)
    }
    fn pause(&self, kind: DebuggerPauseKind) -> DebuggerResumeAction {
        self.debugger().lock().unwrap().pause(kind)
    }
    fn data<'c, 'a>(&'c self) -> DebuggerContextData<'c, 'a> {
        self.data.lock().unwrap()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl wasmtime_runtime::debugger::PatchableCode for EngineDebuggerContext {
    fn patch_jit_code(&self, addr: usize, len: usize, f: &mut dyn FnMut()) {
        let engine_inner = self.engine.upgrade().unwrap();
        let compiled = engine_inner
            .jit_code()
            .lookup_jit_code_range(addr)
            .and_then(|(_, _, module)| module.upgrade())
            .expect("jit_code_range module ref exist");
        compiled.patch_jit_code(addr, len, f);
    }
}
