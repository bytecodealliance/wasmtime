use std::error::Error;
use std::fmt;
use wasmtime_environ::entity::{EntityRef, PrimaryMap};
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::VMFunctionBody;

cfg_if::cfg_if! {
    if #[cfg(all(feature = "jitdump", target_os = "linux"))] {
        #[path = "jitdump_linux.rs"]
        mod jitdump;
    } else {
        #[path = "jitdump_disabled.rs"]
        mod jitdump;
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "vtune", target_os = "linux"))] {
        #[path = "vtune_linux.rs"]
        mod vtune;
    } else {
        #[path = "vtune_disabled.rs"]
        mod vtune;
    }
}

pub use crate::jitdump::JitDumpAgent;
pub use crate::vtune::VTuneAgent;

/// Common interface for profiling tools.
pub trait ProfilingAgent: Send + Sync + 'static {
    /// Notify the profiler of a new module loaded into memory
    fn module_load(
        &self,
        module: &Module,
        functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
        dbg_image: Option<&[u8]>,
    ) -> ();
}

/// Default agent for unsupported profiling build.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullProfilerAgent;

#[derive(Debug)]
struct NullProfilerAgentError;

impl fmt::Display for NullProfilerAgentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A profiler agent is not supported by this build")
    }
}

// This is important for other errors to wrap this one.
impl Error for NullProfilerAgentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl ProfilingAgent for NullProfilerAgent {
    fn module_load(
        &self,
        _module: &Module,
        _functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
        _dbg_image: Option<&[u8]>,
    ) -> () {
    }
}

#[allow(dead_code)]
fn debug_name(module: &Module, index: DefinedFuncIndex) -> String {
    let index = module.local.func_index(index);
    match module.func_names.get(&index) {
        Some(s) => s.clone(),
        None => format!("wasm::wasm-function[{}]", index.index()),
    }
}
