use crate::{CompiledModule, ProfilingAgent};
use anyhow::{bail, Result};

/// Interface for driving the creation of jitdump files
#[derive(Debug)]
pub struct PerfMapAgent {
    _private: (),
}

impl PerfMapAgent {
    /// Intialize a dummy PerfMapAgent that will fail upon instantiation.
    pub fn new() -> Result<Self> {
        bail!("perfmap support not supported on this platform");
    }
}

impl ProfilingAgent for PerfMapAgent {
    fn module_load(&self, _module: &CompiledModule, _dbg_image: Option<&[u8]>) {}
    fn load_single_trampoline(
        &self,
        _name: &str,
        _addr: *const u8,
        _size: usize,
        __pid: u32,
        _tid: u32,
    ) {
    }
}
