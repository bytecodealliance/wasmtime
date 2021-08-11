//! Module for Windows x64 ABI unwind registry.

use anyhow::{bail, Result};
use std::mem;
use winapi::um::winnt;

/// Represents a registry of function unwind information for Windows x64 ABI.
pub struct UnwindRegistration {
    functions: usize,
}

impl UnwindRegistration {
    pub unsafe fn new(
        base_address: *mut u8,
        unwind_info: *mut u8,
        unwind_len: usize,
    ) -> Result<UnwindRegistration> {
        assert!(unwind_info as usize % 4 == 0);
        let unit_len = mem::size_of::<winnt::RUNTIME_FUNCTION>();
        assert!(unwind_len % unit_len == 0);
        if winnt::RtlAddFunctionTable(
            unwind_info as *mut _,
            (unwind_len / unit_len) as u32,
            base_address as u64,
        ) == 0
        {
            bail!("failed to register function table");
        }

        Ok(UnwindRegistration {
            functions: unwind_info as usize,
        })
    }

    pub fn section_name() -> &'static str {
        "_wasmtime_winx64_unwind"
    }
}

impl Drop for UnwindRegistration {
    fn drop(&mut self) {
        unsafe {
            winnt::RtlDeleteFunctionTable(self.functions as _);
        }
    }
}
