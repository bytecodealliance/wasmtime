use std::error::Error;
use std::fmt;

#[cfg(feature = "jitdump")]
mod jitdump;

#[cfg(feature = "jitdump")]
pub use crate::jitdump::JitDumpAgent;

#[cfg(not(feature = "jitdump"))]
pub type JitDumpAgent = NullProfilerAgent;

/// Select which profiling technique to use
#[derive(Debug, Clone, Copy)]
pub enum ProfilingStrategy {
    /// No profiler support
    NullProfiler,

    /// Collect profile for jitdump file format
    JitDumpProfiler,
}

/// Common interface for profiling tools.
pub trait ProfilingAgent {
    /// Notify the profiler of a new module loaded into memory
    fn module_load(
        &mut self,
        module_name: &str,
        addr: *const u8,
        len: usize,
        dbg_image: Option<&[u8]>,
    ) -> ();
}

/// Default agent for unsupported profiling build.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullProfilerAgent {}

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
        &mut self,
        _module_name: &str,
        _addr: *const u8,
        _len: usize,
        _dbg_image: Option<&[u8]>,
    ) -> () {
    }
}
