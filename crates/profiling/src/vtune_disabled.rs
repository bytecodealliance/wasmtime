use crate::ProfilingAgent;
use anyhow::{bail, Result};
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::wasm::DefinedFuncIndex;
use wasmtime_environ::Module;
use wasmtime_runtime::VMFunctionBody;

/// Interface for driving vtune support
#[derive(Debug)]
pub struct VTuneAgent {
    _private: (),
}

impl VTuneAgent {
    /// Intialize a VTuneAgent and write out the header
    pub fn new() -> Result<Self> {
        if cfg!(feature = "vtune") {
            bail!("VTune is not supported on this platform.");
        } else {
            bail!("VTune support disabled at compile time.");
        }
    }
}

impl ProfilingAgent for VTuneAgent {
    fn module_load(
        &self,
        _module: &Module,
        _functions: &PrimaryMap<DefinedFuncIndex, *mut [VMFunctionBody]>,
        _dbg_image: Option<&[u8]>,
    ) {
    }
}
