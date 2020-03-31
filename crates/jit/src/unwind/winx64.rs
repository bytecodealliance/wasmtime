//! Module for Windows x64 ABI unwind registry.

use anyhow::{bail, Result};
use cranelift_codegen::isa::{unwind::UnwindInfo, TargetIsa};
use winapi::um::winnt;

/// Represents a registry of function unwind information for Windows x64 ABI.
pub struct UnwindRegistry {
    base_address: usize,
    functions: Vec<winnt::RUNTIME_FUNCTION>,
    published: bool,
}

impl UnwindRegistry {
    /// Creates a new unwind registry with the given base address.
    pub fn new(base_address: usize) -> Self {
        Self {
            base_address,
            functions: Vec::new(),
            published: false,
        }
    }

    /// Registers a function given the start offset, length, and unwind information.
    pub fn register(&mut self, func_start: u32, func_len: u32, info: &UnwindInfo) -> Result<()> {
        if self.published {
            bail!("unwind registry has already been published");
        }

        match info {
            UnwindInfo::WindowsX64(_) => {
                let mut entry = winnt::RUNTIME_FUNCTION::default();

                entry.BeginAddress = func_start;
                entry.EndAddress = func_start + func_len;

                // The unwind information should be immediately following the function
                // with padding for 4 byte alignment
                unsafe {
                    *entry.u.UnwindInfoAddress_mut() = ((entry.EndAddress + 3) & !3);
                }

                self.functions.push(entry);

                Ok(())
            }
            _ => bail!("unsupported unwind information"),
        }
    }

    /// Publishes all registered functions.
    pub fn publish(&mut self, isa: &dyn TargetIsa) -> Result<()> {
        if self.published {
            bail!("unwind registry has already been published");
        }

        self.published = true;

        if !self.functions.is_empty() {
            // Windows heap allocations are 32-bit aligned, but assert just in case
            assert_eq!(
                (self.functions.as_mut_ptr() as u64) % 4,
                0,
                "function table allocation was not aligned"
            );

            unsafe {
                if winnt::RtlAddFunctionTable(
                    self.functions.as_mut_ptr(),
                    self.functions.len() as u32,
                    base_address as u64,
                ) == 0
                {
                    bail!("failed to register function table");
                }
            }
        }

        Ok(())
    }
}

impl Drop for UnwindRegistry {
    fn drop(&mut self) {
        if self.published {
            unsafe {
                winnt::RtlDeleteFunctionTable(self.functions.as_mut_ptr());
            }
        }
    }
}
