//! Module for Windows x64 ABI unwind registry.

use anyhow::{bail, Result};
use std::mem;
use windows_sys::Win32::System::Diagnostics::Debug::*;

/// Represents a registry of function unwind information for Windows x64 ABI.
pub struct UnwindRegistration {
    functions: usize,
}

impl UnwindRegistration {
    #[allow(missing_docs)]
    pub const SECTION_NAME: &'static str = ".pdata";

    #[allow(missing_docs)]
    pub unsafe fn new(
        base_address: *const u8,
        unwind_info: *const u8,
        unwind_len: usize,
    ) -> Result<UnwindRegistration> {
        assert!(unwind_info as usize % 4 == 0);
        let unit_len = mem::size_of::<IMAGE_RUNTIME_FUNCTION_ENTRY>();
        assert!(unwind_len % unit_len == 0);
        if RtlAddFunctionTable(
            unwind_info as *mut _,
            (unwind_len / unit_len) as u32,
            base_address as _,
        ) == 0
        {
            bail!("failed to register function table");
        }

        Ok(UnwindRegistration {
            functions: unwind_info as usize,
        })
    }
}

impl Drop for UnwindRegistration {
    fn drop(&mut self) {
        unsafe {
            RtlDeleteFunctionTable(self.functions as _);
        }
    }
}
