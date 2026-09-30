use std::ffi::c_void;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub use std::io::Result;

#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub use wasmtime_core::error::Result;

#[cfg(all(
    target_arch = "aarch64",
    any(target_os = "linux", target_os = "android")
))]
mod details {

    use super::*;
    use libc::{EINVAL, EPERM, syscall};
    use std::io::Error;

    const MEMBARRIER_CMD_GLOBAL: libc::c_int = 1;
    const MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE: libc::c_int = 32;
    const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE: libc::c_int = 64;

    /// See docs on [crate::pipeline_flush_mt] for a description of what this function is trying to do.
    #[inline]
    pub(crate) fn pipeline_flush_mt() -> Result<()> {
        // Ensure that no processor has fetched a stale instruction stream.
        //
        // On AArch64 we try to do this by executing a "broadcast" `ISB` which is not something
        // that the architecture provides us but we can emulate it using the membarrier kernel
        // interface.
        //
        // This behaviour was documented in a patch, however it seems that it hasn't been
        // upstreamed yet Nevertheless it clearly explains the guarantees that the Linux kernel
        // provides us regarding the membarrier interface, and how to use it for JIT contexts.
        // https://lkml.kernel.org/lkml/07a8b963002cb955b7516e61bad19514a3acaa82.1623813516.git.luto@kernel.org/
        //
        // I couldn't find the follow up for that patch but there doesn't seem to be disagreement
        // about that specific part in the replies.
        // TODO: Check if the kernel has updated the membarrier documentation
        //
        // See the following issues for more info:
        //  * https://github.com/bytecodealliance/wasmtime/pull/3426
        //  * https://github.com/bytecodealliance/wasmtime/pull/4997
        //
        // TODO: x86 and s390x have coherent caches so they don't need this, but RISCV does not
        // guarantee that, so we may need to do something similar for it. However as noted in the
        // above kernel patch the SYNC_CORE membarrier has different guarantees on each
        // architecture so we need follow up and check what it provides us.
        // See: https://github.com/bytecodealliance/wasmtime/issues/5033
        match membarrier(MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE) {
            Ok(_) => {}

            // EPERM happens if the calling process hasn't yet called the register membarrier.
            // We can call the register membarrier now, and then retry the actual membarrier,
            //
            // This does have some overhead since on the first time we call this function we
            // actually execute three membarriers, but this only happens once per process and only
            // one slow membarrier is actually executed (The last one, which actually generates an
            // IPI).
            Err(e) if e.raw_os_error().unwrap() == EPERM => {
                membarrier(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE)?;
                membarrier(MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE)?;
            }

            // On kernels older than 4.16 the above syscall does not exist, so we can
            // fallback to MEMBARRIER_CMD_GLOBAL which is an alias for MEMBARRIER_CMD_SHARED
            // that has existed since 4.3. GLOBAL is a lot slower, but allows us to have
            // compatibility with older kernels.
            Err(e) if e.raw_os_error().unwrap() == EINVAL => {
                membarrier(MEMBARRIER_CMD_GLOBAL)?;
            }

            // In any other case we got an actual error, so lets propagate that up
            e => e?,
        }

        Ok(())
    }

    fn membarrier(barrier: libc::c_int) -> Result<()> {
        let flags: libc::c_int = 0;
        let res = unsafe { syscall(libc::SYS_membarrier, barrier, flags) };
        if res == 0 {
            Ok(())
        } else {
            Err(Error::last_os_error())
        }
    }
}

#[cfg(not(all(
    target_arch = "aarch64",
    any(target_os = "linux", target_os = "android")
)))]
mod details {
    // NB: this uses `wasmtime_environ::error::Result` instead of `std::io::Result` to compile on
    // `no_std`.
    pub(crate) fn pipeline_flush_mt() -> super::Result<()> {
        Ok(())
    }
}

#[cfg(all(target_arch = "riscv64", target_os = "linux"))]
fn riscv_flush_icache(start: u64, end: u64) -> Result<()> {
    cfg_select! {
        feature = "one-core" => {
            use std::arch::asm;
            let _ = (start, end);
            unsafe {
                asm!("fence.i");
            };
            Ok(())
        }
        _ => {
            #[expect(non_upper_case_globals, reason = "matching C style")]
            match unsafe {
                libc::syscall(
                    {
                        // The syscall isn't defined in `libc`, so we define the syscall number here.
                        // https://github.com/torvalds/linux/search?q=__NR_arch_specific_syscall
                        const  __NR_arch_specific_syscall :i64 = 244;
                        // https://github.com/torvalds/linux/blob/5bfc75d92efd494db37f5c4c173d3639d4772966/tools/arch/riscv/include/uapi/asm/unistd.h#L40
                        const sys_riscv_flush_icache :i64 =  __NR_arch_specific_syscall + 15;
                        sys_riscv_flush_icache
                    },
                    // Currently these parameters are not used, but they are still defined.
                    start, // start
                    end, // end
                    {
                        const SYS_RISCV_FLUSH_ICACHE_LOCAL :i64 = 1;
                        const SYS_RISCV_FLUSH_ICACHE_ALL :i64 = SYS_RISCV_FLUSH_ICACHE_LOCAL;
                        SYS_RISCV_FLUSH_ICACHE_ALL
                    }, // flags
                )
            } {
                0 => { Ok(()) }
                _ => Err(std::io::Error::last_os_error()),
            }
        }
    }
}

#[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
fn aarch64_flush_icache(start: u64, end: u64) {
    // Reading CTR_EL0 at EL0 gets SIGILL on macOS, so do what compiler-rt does on
    // Darwin and call the libSystem routine instead:
    // https://github.com/llvm/llvm-project/blob/3390613ccf0fbbb40026dcbabb36fce444f79480/compiler-rt/lib/builtins/clear_cache.c#L225-L228
    // https://github.com/apple/darwin-libplatform/blob/215b09856ab5765b7462a91be7076183076600df/src/cachecontrol/arm64/cache.s#L31-L52
    unsafe extern "C" {
        fn sys_icache_invalidate(start: *mut c_void, len: usize);
    }
    unsafe {
        sys_icache_invalidate(start as *mut c_void, (end - start) as usize);
    }
}

/// The fields of CTR_EL0 that decide which cache maintenance a core needs.
#[cfg(any(all(target_arch = "aarch64", not(target_vendor = "apple")), test))]
struct CacheType {
    idc: bool,
    dic: bool,
    dmin_line: u64,
    imin_line: u64,
}

#[cfg(any(all(target_arch = "aarch64", not(target_vendor = "apple")), test))]
impl CacheType {
    fn decode(ctr: u64) -> Self {
        CacheType {
            idc: ctr & (1 << 28) != 0,
            dic: ctr & (1 << 29) != 0,
            dmin_line: 4 << ((ctr >> 16) & 15),
            imin_line: 4 << (ctr & 15),
        }
    }
}

#[cfg(all(target_arch = "aarch64", not(target_vendor = "apple")))]
fn aarch64_flush_icache(start: u64, end: u64) {
    use core::arch::asm;
    use core::sync::atomic::{AtomicU64, Ordering::Relaxed};

    // The sequence compiler-rt and the Linux kernel use to make newly written
    // instructions visible: clean the data cache to the point of unification (unless
    // CTR_EL0.IDC says that isn't needed), then invalidate the instruction cache (unless
    // CTR_EL0.DIC), each by that cache's minimum line size. Without the clean, `ic ivau`
    // can refetch stale bytes on cores such as Cortex-A72.
    // https://github.com/llvm/llvm-project/blob/3390613ccf0fbbb40026dcbabb36fce444f79480/compiler-rt/lib/builtins/clear_cache.c#L121-L153
    // https://github.com/torvalds/linux/blob/551c722f40809618230001baccf219193e22fc5a/arch/arm64/mm/cache.S#L28-L43

    // Bit 31 of CTR_EL0 is RES1, so zero means it hasn't been read yet.
    static CTR_EL0: AtomicU64 = AtomicU64::new(0);
    let mut ctr = CTR_EL0.load(Relaxed);
    if ctr == 0 {
        unsafe {
            asm!("mrs {}, ctr_el0", out(reg) ctr, options(nomem, nostack, preserves_flags));
        }
        CTR_EL0.store(ctr, Relaxed);
    }

    let cache = CacheType::decode(ctr);
    if !cache.idc {
        let mut addr = start & !(cache.dmin_line - 1);
        while addr < end {
            unsafe {
                asm!("dc cvau, {}", in(reg) addr, options(nostack, preserves_flags));
            }
            addr += cache.dmin_line;
        }
    }
    unsafe {
        asm!("dsb ish", options(nostack, preserves_flags));
    }

    if !cache.dic {
        let mut addr = start & !(cache.imin_line - 1);
        while addr < end {
            unsafe {
                asm!("ic ivau, {}", in(reg) addr, options(nostack, preserves_flags));
            }
            addr += cache.imin_line;
        }
        unsafe {
            asm!("dsb ish", options(nostack, preserves_flags));
        }
    }
    unsafe {
        asm!("isb", options(nostack, preserves_flags));
    }
}

pub(crate) use details::*;

/// See docs on [crate::clear_cache] for a description of what this function is trying to do.
#[inline]
pub(crate) fn clear_cache(_ptr: *const c_void, _len: usize) -> Result<()> {
    #[cfg(target_arch = "aarch64")]
    aarch64_flush_icache(_ptr as u64, (_ptr as u64) + (_len as u64));
    #[cfg(all(target_arch = "riscv64", target_os = "linux"))]
    riscv_flush_icache(_ptr as u64, (_ptr as u64) + (_len as u64))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn ctr_el0_decode() {
        // Cortex-A72 (Graviton1), qemu -cpu max, Neoverse-N1 (Graviton2)
        for (ctr, idc, dic, dmin_line, imin_line) in [
            (0x8444c004, false, false, 64, 64),
            (0x80038003, false, false, 32, 32),
            (0xb444c004, true, true, 64, 64),
        ] {
            let cache = super::CacheType::decode(ctr);
            assert_eq!(cache.idc, idc, "{ctr:#x}");
            assert_eq!(cache.dic, dic, "{ctr:#x}");
            assert_eq!(cache.dmin_line, dmin_line, "{ctr:#x}");
            assert_eq!(cache.imin_line, imin_line, "{ctr:#x}");
        }
    }

    // Rewrite a function in a page that has already executed it, the way a JIT reuses
    // code memory. Cores with CTR_EL0.IDC == 0 run the old function without the data
    // cache clean; the kernel only cleans a page the first time it becomes executable.
    #[cfg(all(
        target_arch = "aarch64",
        any(
            target_os = "linux",
            target_os = "android",
            target_os = "freebsd",
            target_os = "macos"
        )
    ))]
    #[test]
    fn rewritten_code_runs() {
        use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE};
        use std::ptr::null_mut;

        let len = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let page = unsafe {
            libc::mmap(
                null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(page, libc::MAP_FAILED);
        let code = page as *mut u32;
        let f: extern "C" fn() -> u32 = unsafe { std::mem::transmute(code) };
        for i in 0..100u32 {
            unsafe {
                assert_eq!(libc::mprotect(page, len, PROT_READ | PROT_WRITE), 0);
                code.write_volatile(0x5280_0000 | (i << 5)); // movz w0, #i
                code.add(1).write_volatile(0xd65f_03c0); // ret
                crate::clear_cache(page, 8).unwrap();
                assert_eq!(libc::mprotect(page, len, PROT_READ | PROT_EXEC), 0);
            }
            crate::pipeline_flush_mt().unwrap();
            assert_eq!(f(), i);
        }
        unsafe {
            libc::munmap(page, len);
        }
    }
}
