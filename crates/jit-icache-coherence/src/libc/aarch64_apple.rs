use std::ffi::c_void;
use wasmtime_core::error::Result;

/// See docs on [crate::clear_cache] for a description of what this function is trying to do.
pub(crate) fn clear_cache(ptr: *const c_void, len: usize) -> Result<()> {
    // Reading CTR_EL0 at EL0 gets SIGILL on macOS, so do what compiler-rt does on
    // Darwin and call the libSystem routine. It also carries Apple's own workarounds:
    // since libplatform-306 (macOS 14) it issues an extra `dsb ish` after every 20
    // `ic ivau` on the CPU families in `cpus_that_need_dsb_for_ic_ivau`, which an
    // inline copy of the sequence would miss.
    // https://github.com/llvm/llvm-project/blob/3390613ccf0fbbb40026dcbabb36fce444f79480/compiler-rt/lib/builtins/clear_cache.c#L225-L228
    // https://github.com/apple-oss-distributions/libplatform/blob/libplatform-375.120.2/src/cachecontrol/arm64/cache.s#L31-L92
    unsafe extern "C" {
        fn sys_icache_invalidate(start: *mut c_void, len: usize);
    }
    unsafe {
        sys_icache_invalidate(ptr as *mut c_void, len);
    }
    Ok(())
}
