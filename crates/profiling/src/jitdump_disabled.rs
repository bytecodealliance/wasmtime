use crate::ProfilingAgent;
use anyhow::{bail, Result};
use wasmtime_environ::{DefinedFuncIndex, Module, PrimaryMap};
use wasmtime_runtime::VMFunctionBody;

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
    fn module_load(
        &self,
        _module: &Module,
        _functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
        _dbg_image: Option<&[u8]>,
    ) {
    }
}
