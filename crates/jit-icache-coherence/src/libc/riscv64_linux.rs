use std::ffi::c_void;
use wasmtime_core::error::Result;

/// See docs on [crate::clear_cache] for a description of what this function is trying to do.
pub(crate) fn clear_cache(ptr: *const c_void, len: usize) -> Result<()> {
    riscv_flush_icache(ptr as u64, (ptr as u64) + (len as u64))
}

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
                _ => Err(std::io::Error::last_os_error().into()),
            }
        }
    }
}
