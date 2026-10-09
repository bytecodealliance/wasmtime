//! Rewrite a function in a page that has already executed it, the way a JIT reuses
//! code memory. Cores with CTR_EL0.IDC == 0 run the old function without the data
//! cache clean; the kernel only cleans a page the first time it becomes executable.

#![cfg(all(
    not(miri),
    target_arch = "aarch64",
    any(
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd",
        target_os = "macos"
    )
))]

use wasmtime_internal_jit_icache_coherence as icache;

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
            icache::clear_cache(page, 8).unwrap();
            assert_eq!(libc::mprotect(page, len, PROT_READ | PROT_EXEC), 0);
        }
        icache::pipeline_flush_mt().unwrap();
        assert_eq!(f(), i);
    }
    unsafe {
        libc::munmap(page, len);
    }
}
