use std::ffi::c_void;
use std::io::Result;

/// See docs on [crate::pipeline_flush_mt] for a description of what this function is trying to do.
#[inline]
pub(crate) fn pipeline_flush_mt() -> Result<()> {
    // Note: If you are changing anything in this function, please make sure to update
    // [libc::pipeline_flush_mt] as well. These two functions are trying to do the same thing
    // and should be kept in sync.

    // Ensure that no processor has fetched a stale instruction stream.
    //
    // On AArch64 we try to do this by executing a "broadcast" `ISB` which is not something that the
    // architecture provides us but we can emulate it using the membarrier kernel interface.
    //
    // This behaviour was documented in a patch, however it seems that it hasn't been upstreamed yet
    // Nevertheless it clearly explains the guarantees that the Linux kernel provides us regarding the
    // membarrier interface, and how to use it for JIT contexts.
    // https://lkml.kernel.org/lkml/07a8b963002cb955b7516e61bad19514a3acaa82.1623813516.git.luto@kernel.org/
    //
    // I couldn't find the follow up for that patch but there doesn't seem to be disagreement about
    // that specific part in the replies.
    // TODO: Check if the kernel has updated the membarrier documentation
    //
    // See the following issues for more info:
    //  * https://github.com/bytecodealliance/wasmtime/pull/3426
    //  * https://github.com/bytecodealliance/wasmtime/pull/4997
    //
    // TODO: x86 and s390x have coherent caches so they don't need this, but RISCV does not
    // guarantee that, so we may need to do something similar for it. However as noted in the above
    // kernel patch the SYNC_CORE membarrier has different guarantees on each architecture
    // so we need follow up and check what it provides us.
    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    {
        use rustix::io::Errno;
        use rustix::process::{membarrier, MembarrierCommand};

        match membarrier(MembarrierCommand::PrivateExpeditedSyncCore) {
            Ok(_) => {}

            // EPERM happens if the calling process hasn't yet called the register membarrier.
            // We can call the register membarrier now, and then retry the actual membarrier,
            //
            // This does have some overhead since on the first time we call this function we
            // actually execute three membarriers, but this only happens once per process and only
            // one slow membarrier is actually executed (The last one, which actually generates an IPI).
            Err(Errno::PERM) => {
                membarrier(MembarrierCommand::RegisterPrivateExpeditedSyncCore)?;
                membarrier(MembarrierCommand::PrivateExpeditedSyncCore)?;
            }

            // On kernels older than 4.16 the above syscall does not exist, so we can
            // fallback to MEMBARRIER_CMD_GLOBAL which is an alias for MEMBARRIER_CMD_SHARED
            // that has existed since 4.3. GLOBAL is a lot slower, but allows us to have
            // compatibility with older kernels.
            Err(Errno::INVAL) => membarrier(MembarrierCommand::Global)?,

            // In any other case we got an actual error, so lets propagate that up
            e => e?,
        }
    }

    Ok(())
}

/// See docs on [crate::clear_cache] for a description of what this function is trying to do.
#[inline]
pub(crate) fn clear_cache(_ptr: *const c_void, _len: usize) -> Result<()> {
    // TODO: On AArch64 we currently rely on the `mprotect` call that switches the memory from W+R to R+X
    // to do this for us, however that is an implementation detail and should not be relied upon
    // We should call some implementation of `clear_cache` here
    //
    // See: https://github.com/bytecodealliance/wasmtime/issues/3310

    Ok(())
}
