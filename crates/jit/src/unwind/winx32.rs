//! Stub unwind registry for Windows x32.

use anyhow::{bail, Result};
use cranelift_codegen::isa::{unwind::UnwindInfo, TargetIsa};

pub struct UnwindRegistry {}

impl UnwindRegistry {
    pub fn new(_base_address: usize) -> Self {
        Self {}
    }

    pub fn register(&mut self, _func_start: u32, _func_len: u32, _info: &UnwindInfo) -> Result<()> {
        bail!("winx32 has no unwind registry")
    }

    pub fn publish(&mut self, _isa: &dyn TargetIsa) -> Result<()> {
        Ok(())
    }
}
