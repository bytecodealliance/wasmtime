use crate::CompiledModule;
use std::error::Error;
use std::fmt;
use wasmtime_environ::{DefinedFuncIndex, EntityRef, Module};

cfg_if::cfg_if! {
    if #[cfg(all(feature = "jitdump", target_os = "linux"))] {
        #[path = "profiling/jitdump_linux.rs"]
        mod jitdump;
    } else {
        #[path = "profiling/jitdump_disabled.rs"]
        mod jitdump;
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "vtune", target_os = "linux"))] {
        #[path = "profiling/vtune_linux.rs"]
        mod vtune;
    } else {
        #[path = "profiling/vtune_disabled.rs"]
        mod vtune;
    }
}

pub use jitdump::JitDumpAgent;
pub use vtune::VTuneAgent;

/// Common interface for profiling tools.
pub trait ProfilingAgent: Send + Sync + 'static {
    /// Notify the profiler of a new module loaded into memory
    fn module_load(&self, module: &CompiledModule, dbg_image: Option<&[u8]>);
    /// Notify the profiler that the object file provided contains
    /// dynamically-generated trampolines which are now being loaded.
    fn trampoline_load(&self, file: &object::File<'_>);
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
    fn module_load(&self, _module: &CompiledModule, _dbg_image: Option<&[u8]>) {}
    fn trampoline_load(&self, _file: &object::File<'_>) {}
}

#[allow(dead_code)]
fn debug_name(module: &Module, index: DefinedFuncIndex) -> String {
    let index = module.func_index(index);
    match module.func_names.get(&index) {
        Some(s) => rustc_demangle::try_demangle(s)
            .map(|demangle| demangle.to_string())
            .or_else(|_| cpp_demangle::Symbol::new(s).map(|sym| sym.to_string()))
            .unwrap_or_else(|_| s.clone()),
        None => format!("wasm::wasm-function[{}]", index.index()),
    }
}
