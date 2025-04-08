use cranelift_module::{ModuleError, ModuleResult};
use std::io;

mod arena;
mod system;

pub use arena::ArenaMemoryProvider;
pub use system::SystemMemoryProvider;

/// Type of branch protection to apply to executable memory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BranchProtection {
    /// No protection.
    None,
    /// Use the Branch Target Identification extension of the Arm architecture.
    BTI,
}

/// A provider of memory for the JIT.
pub trait JITMemoryProvider {
    /// Allocate memory that will be executable once finalized.
    fn allocate_readexec(&mut self, size: usize, align: u64) -> io::Result<*mut u8>;
    /// Allocate writable memory.
    fn allocate_readwrite(&mut self, size: usize, align: u64) -> io::Result<*mut u8>;
    /// Allocate memory that will be read-only once finalized.
    fn allocate_readonly(&mut self, size: usize, align: u64) -> io::Result<*mut u8>;

    /// Free the memory region.
    unsafe fn free_memory(&mut self);
    /// Finalize the memory region and apply memory protections.
    fn finalize(&mut self, branch_protection: BranchProtection) -> ModuleResult<()>;
}

/// Marks the memory region as readable and executable.
///
/// This function deals with applies branch protection and clears the icache,
/// but *doesn't* flush the pipeline. Callers have to ensure that
/// [`wasmtime_jit_icache_coherence::pipeline_flush_mt`] is called before the
/// mappings are used.
pub(crate) fn set_readable_and_executable(
    ptr: *mut u8,
    len: usize,
    branch_protection: BranchProtection,
) -> ModuleResult<()> {
    // Clear all the newly allocated code from cache if the processor requires it
    //
    // Do this before marking the memory as R+X, technically we should be able to do it after
    // but there are some CPU's that have had errata about doing this with read only memory.
    unsafe {
        wasmtime_jit_icache_coherence::clear_cache(ptr as *const libc::c_void, len)
            .expect("Failed cache clear")
    };

    unsafe {
        region::protect(ptr, len, region::Protection::READ_EXECUTE).map_err(|e| {
            ModuleError::Backend(
                anyhow::Error::new(e).context("unable to make memory readable+executable"),
            )
        })?;
    }

    // If BTI is requested, and the architecture supports it, use mprotect to set the PROT_BTI flag.
    if branch_protection == BranchProtection::BTI {
        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        if std::arch::is_aarch64_feature_detected!("bti") {
            let prot = libc::PROT_EXEC | libc::PROT_READ | /* PROT_BTI */ 0x10;

            unsafe {
                if libc::mprotect(ptr as *mut libc::c_void, len, prot) < 0 {
                    return Err(ModuleError::Backend(
                        anyhow::Error::new(io::Error::last_os_error())
                            .context("unable to make memory readable+executable"),
                    ));
                }
            }
        }
    }

    Ok(())
}
