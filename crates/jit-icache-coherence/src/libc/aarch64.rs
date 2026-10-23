use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::ffi::c_void;
use wasmtime_core::error::Result;

/// See docs on [crate::clear_cache] for a description of what this function is trying to do.
pub(crate) fn clear_cache(ptr: *const c_void, len: usize) -> Result<()> {
    flush_icache(ptr as u64, (ptr as u64) + (len as u64));
    Ok(())
}

/// The fields of CTR_EL0 that decide which cache maintenance a core needs.
struct CacheType {
    idc: bool,
    dic: bool,
    dmin_line: u64,
    imin_line: u64,
}

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

/// The start address of every `line`-byte cache line that overlaps `start..end`.
fn cache_lines(start: u64, end: u64, line: u64) -> impl Iterator<Item = u64> {
    (start - start % line..end).step_by(line as usize)
}

fn flush_icache(start: u64, end: u64) {
    // Make newly written instructions visible as the Arm ARM describes it (DDI 0487,
    // "Concurrent modification and execution of instructions"), skipping the steps
    // CTR_EL0 says a core doesn't need: clean the data cache to the point of
    // unification unless IDC, `dsb ish`, invalidate the instruction cache unless DIC,
    // `dsb ish`, `isb`, each loop by that cache's minimum line size. Without the clean,
    // `ic ivau` can refetch stale bytes on cores such as Cortex-A72. This is
    // compiler-rt's `__clear_cache` sequence; the Linux kernel's
    // `caches_clean_inval_pou_macro` differs only in using `dsb ishst` when IDC is set.
    // https://github.com/llvm/llvm-project/blob/3390613ccf0fbbb40026dcbabb36fce444f79480/compiler-rt/lib/builtins/clear_cache.c#L121-L153
    // https://github.com/torvalds/linux/blob/551c722f40809618230001baccf219193e22fc5a/arch/arm64/mm/cache.S#L28-L43
    //
    // CTR_EL0 is read once per process although threads run on, and migrate between,
    // different cores. IDC and DIC must be the same on every core in an Inner Shareable
    // domain (DDI 0487, CTR_EL0 description), which leaves the line sizes. Linux, NetBSD
    // 10 and FreeBSD 15 make those uniform too: a core whose CTR_EL0 differs from the
    // system-wide safe value traps EL0 reads and gets that value, with the smallest
    // line sizes. Reading it on every call would give the same answer, and could not
    // help on other systems anyway, since a thread can migrate between the read and
    // the loops.
    // https://github.com/torvalds/linux/blob/551c722f40809618230001baccf219193e22fc5a/arch/arm64/kernel/cpu_errata.c#L172-L189
    // https://github.com/torvalds/linux/blob/551c722f40809618230001baccf219193e22fc5a/arch/arm64/kernel/traps.c#L601-L618

    // Bit 31 of CTR_EL0 is RES1, so zero means it hasn't been read yet.
    static CTR_EL0: AtomicU64 = AtomicU64::new(0);
    let mut ctr = CTR_EL0.load(Relaxed);
    if ctr == 0 {
        unsafe {
            asm!("mrs {}, ctr_el0", out(reg) ctr, options(nomem, nostack, preserves_flags));
        }
        CTR_EL0.store(ctr, Relaxed);
    }

    // None of the blocks below may be `nomem`: the stores of the new code have to stay
    // ahead of the maintenance and the barriers.
    let cache = CacheType::decode(ctr);
    if !cache.idc {
        for addr in cache_lines(start, end, cache.dmin_line) {
            unsafe {
                asm!("dc cvau, {}", in(reg) addr, options(nostack, preserves_flags));
            }
        }
    }
    unsafe {
        asm!("dsb ish", options(nostack, preserves_flags));
    }

    if !cache.dic {
        for addr in cache_lines(start, end, cache.imin_line) {
            unsafe {
                asm!("ic ivau, {}", in(reg) addr, options(nostack, preserves_flags));
            }
        }
        unsafe {
            asm!("dsb ish", options(nostack, preserves_flags));
        }
    }
    unsafe {
        asm!("isb", options(nostack, preserves_flags));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ctr_el0_decode() {
        for (ctr, idc, dic, dmin_line, imin_line) in [
            // Cortex-A72 (Graviton1)
            (0x8444c004, false, false, 64, 64),
            // qemu -cpu max
            (0x80038003, false, false, 32, 32),
            // Neoverse-N1 (Graviton2)
            (0xb444c004, true, true, 64, 64),
            // Cortex-A55: IDC without DIC
            (0x94448004, true, false, 64, 64),
            // A64FX: 256-byte lines
            (0x86668006, false, false, 256, 256),
            // What Linux reports on a Neoverse-N1 with erratum 1542419: DIC hidden and
            // IminLine raised to the page size, for 4K and 64K pages
            (0x9444c00a, true, false, 64, 4096),
            (0x9444c00e, true, false, 64, 65536),
        ] {
            let cache = super::CacheType::decode(ctr);
            assert_eq!(cache.idc, idc, "{ctr:#x}");
            assert_eq!(cache.dic, dic, "{ctr:#x}");
            assert_eq!(cache.dmin_line, dmin_line, "{ctr:#x}");
            assert_eq!(cache.imin_line, imin_line, "{ctr:#x}");
        }
    }

    #[test]
    fn cache_lines_cover_the_range() {
        let lines = |start, end, line| super::cache_lines(start, end, line).collect::<Vec<u64>>();
        assert_eq!(lines(0x1000, 0x1008, 64), [0x1000]);
        assert_eq!(lines(0x1030, 0x1050, 64), [0x1000, 0x1040]);
        assert_eq!(lines(0x1fff, 0x2001, 4096), [0x1000, 0x2000]);
        assert!(lines(0x1000, 0x1000, 64).is_empty());
    }
}
