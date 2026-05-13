//! Detection of Memory Protection Keys (MPK) support on the host system.
//!
//! This is the single source of truth for whether Wasmtime's MPK
//! implementation can be used on the current target. The runtime in the
//! `wasmtime` crate consults [`is_supported`], and `examples/mpk-available.rs`
//! exposes it as a CLI exit code so CI can decide whether to set
//! `WASMTIME_TEST_FORCE_MPK=1`.

/// Returns `true` if Wasmtime's MPK support can be used on this host.
pub fn is_supported() -> bool {
    cfg!(target_os = "linux") && cpuid_pku_bit_set()
}

/// Check the `ECX.PKU` flag (bit 3, zero-based) of the `07h` `CPUID` leaf; see
/// the Intel Software Development Manual, vol 3a, section 2.7. This flag is
/// only set on Intel CPUs, so this function also checks the `CPUID` vendor
/// string.
#[cfg(target_arch = "x86_64")]
fn cpuid_pku_bit_set() -> bool {
    is_intel_cpu() && {
        #[allow(
            unused_unsafe,
            reason = "rust is transitioning to `__cpuid` being a safe function"
        )]
        let result = unsafe { core::arch::x86_64::__cpuid(0x07) };
        (result.ecx & 0b1000) != 0
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn cpuid_pku_bit_set() -> bool {
    false
}

/// Check the `CPUID` vendor string for `GenuineIntel`; see the Intel Software
/// Development Manual, vol 2a, `CPUID` description.
#[cfg(target_arch = "x86_64")]
fn is_intel_cpu() -> bool {
    #[allow(unused_unsafe, reason = "see above about __cpuid")]
    let result = unsafe { core::arch::x86_64::__cpuid(0) };
    result.ebx == u32::from_le_bytes(*b"Genu")
        && result.edx == u32::from_le_bytes(*b"ineI")
        && result.ecx == u32::from_le_bytes(*b"ntel")
}
