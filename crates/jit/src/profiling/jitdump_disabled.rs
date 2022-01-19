use crate::{CompiledModule, ProfilingAgent};
use anyhow::{bail, Result};

/// Interface for driving the creation of jitdump files
#[derive(Debug)]
pub struct JitDumpAgent {
    _private: (),
}

impl JitDumpAgent {
    /// Intialize a JitDumpAgent and write out the header
    pub fn new() -> Result<Self> {
        if cfg!(feature = "jitdump") {
            bail!("jitdump is not supported on this platform");
        } else {
            bail!("jitdump support disabled at compile time");
        }
    }
}

impl ProfilingAgent for JitDumpAgent {
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
