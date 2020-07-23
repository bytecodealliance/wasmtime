#![allow(missing_docs)]

use crate::runtime::{Engine, EngineInner};
use std::sync::{Arc, Mutex, Weak};

pub use wasmtime_runtime::debugger::{
    BreakpointData, DebuggerContext, DebuggerContextData, DebuggerPauseKind, DebuggerResumeAction,
    PatchableCode,
};

pub trait DebuggerAgent: std::marker::Send + std::marker::Sync {
    fn pause(&mut self, kind: DebuggerPauseKind) -> DebuggerResumeAction;
}

pub(crate) struct NullDebuggerAgent;

impl DebuggerAgent for NullDebuggerAgent {
    fn pause(&mut self, _kind: DebuggerPauseKind) -> DebuggerResumeAction {
        DebuggerResumeAction::Continue
    }
}

pub(crate) struct EngineDebuggerContext {
    engine: Weak<EngineInner>,
    breakpoints: Vec<BreakpointData>,
    data: Mutex<Option<Box<dyn std::any::Any + Send + Sync>>>,
}

fn _assert_engine_debugger_context_send_sync() {
    fn _assert<T: Send + Sync>() {}
    _assert::<EngineDebuggerContext>();
}

impl EngineDebuggerContext {
    pub fn new(engine: &Engine) -> EngineDebuggerContext {
        EngineDebuggerContext {
            engine: engine.clone().weak(),
            breakpoints: Vec::new(),
            data: Mutex::new(None),
        }
    }
    pub fn add_breakpoints(&mut self, it: impl Iterator<Item = BreakpointData>) {
        self.breakpoints.extend(it);
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
    fn find_breakpoint(&self, addr: *const u8) -> Option<&BreakpointData> {
        let addr = addr as usize;
        self.breakpoints.iter().find(|b| b.pc == addr)
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
