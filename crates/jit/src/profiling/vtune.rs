//! Adds support for profiling JIT-ed code using VTune. By default, VTune
//! support is built in to Wasmtime (configure with the `vtune` feature flag).
//! To enable it at runtime, use the `--vtune` CLI flag.
//!
//! ### Profile
//!
//! ```ignore
//! vtune -run-pass-thru=--no-altstack -v -collect hotspots target/debug/wasmtime --vtune test.wasm
//! ```
//!
//! Note: `vtune` is a command-line tool for VTune which must [be
//! installed](https://www.intel.com/content/www/us/en/developer/tools/oneapi/vtune-profiler.html#standalone)
//! for this to work.

use crate::{CompiledModule, ProfilingAgent};
use anyhow::Result;
use ittapi::jit::MethodLoadBuilder;
use std::sync::{atomic, Mutex};
use wasmtime_environ::EntityRef;

/// Interface for driving the ittapi for VTune support
pub struct VTuneAgent {
    // Note that we use a mutex internally to serialize state updates since multiple threads may be
    // sharing this agent.
    state: Mutex<State>,
}

/// Interface for driving vtune
#[derive(Default)]
struct State {
    vtune: ittapi::jit::Jit,
}

impl VTuneAgent {
    /// Initialize a VTuneAgent.
    pub fn new() -> Result<Self> {
        Ok(VTuneAgent {
            state: Mutex::new(State {
                vtune: Default::default(),
            }),
        })
    }
}

impl Drop for VTuneAgent {
    fn drop(&mut self) {
        self.state.lock().unwrap().event_shutdown();
    }
}

impl State {
    /// Notify vtune about a newly tracked code region.
    fn notify_code(&mut self, module_name: &str, method_name: &str, addr: *const u8, len: usize) {
        self.vtune
            .load_method(
                MethodLoadBuilder::new(method_name.to_owned(), addr, len)
                    .class_file_name(module_name.to_owned())
                    .source_file_name("<unknown wasm filename>".to_owned()),
            )
            .unwrap();
    }

    /// Shutdown module
    fn event_shutdown(&mut self) {
        // Ignore if something went wrong.
        let _ = self.vtune.shutdown();
    }
}

impl ProfilingAgent for VTuneAgent {
    fn module_load(&self, module: &CompiledModule, dbg_image: Option<&[u8]>) {
        self.state.lock().unwrap().module_load(module, dbg_image);
    }
    fn load_single_trampoline(&self, name: &str, addr: *const u8, size: usize, pid: u32, tid: u32) {
        self.state
            .lock()
            .unwrap()
            .load_single_trampoline(name, addr, size, pid, tid);
    }
}

impl State {
    fn module_load(&mut self, module: &CompiledModule, _dbg_image: Option<&[u8]>) {
        // Global counter for module ids.
        static MODULE_ID: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
        let global_module_id = MODULE_ID.fetch_add(1, atomic::Ordering::SeqCst);

        let module_name = module
            .module()
            .name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| format!("wasm_module_{}", global_module_id));

        for (idx, func) in module.finished_functions() {
            let (addr, len) = unsafe { ((*func).as_ptr().cast::<u8>(), (*func).len()) };
            let method_name = super::debug_name(module, idx);
            log::trace!(
                "new function {:?}::{:?} @ {:?}\n",
                module_name,
                method_name,
                addr
            );
            self.notify_code(&module_name, &method_name, addr, len);
        }

        // Note: these are the trampolines into exported functions.
        for (idx, func, len) in module.trampolines() {
            let idx = idx.index();
            let (addr, len) = (func as usize as *const u8, len);
            let method_name = format!("wasm::trampoline[{}]", idx,);
            log::trace!(
                "new trampoline for exported signature {} @ {:?}\n",
                idx,
                addr
            );
            self.notify_code(&module_name, &method_name, addr, len);
        }
    }

    fn load_single_trampoline(
        &mut self,
        name: &str,
        addr: *const u8,
        size: usize,
        _pid: u32,
        _tid: u32,
    ) {
        self.notify_code("wasm trampoline for Func::new", name, addr, size);
    }
}
