//! Detection of Memory Protection Keys (MPK) support on the host system.
//!
//! This is the single source of truth for whether Wasmtime's MPK
//! implementation can be used on the current target. The runtime in the
//! `wasmtime` crate consults [`is_supported`], and `examples/mpk-available.rs`
//! exposes it as a CLI exit code so CI can decide whether to set
//! `WASMTIME_TEST_FORCE_MPK=1`.

/// Returns `true` if Wasmtime's MPK support can be used on this host.
pub fn is_supported() -> bool {
    cfg!(target_os = "linux") && cpuid_pku_enabled()
}

/// Check whether protection keys (pkus) are supported by the CPU and enabled by the OS.
///
/// This is exposed in the ECX result of cpuid leaf 07h:
///
/// - bit 3 (`PKU`) is set if the processor supports protection keys;
/// - bit 4 (`OSPKE`) is set if the OS has also set `CR4.PKE`, which is what makes
///   `RDPKRU`/`WRPKRU` usable. Without it these instructions raise `#UD` (`SIGILL`), for example in
///   virtual machines whose hypervisor reports `PKU` but doesn't enable it.
///
/// This is consistent between Intel and AMD CPUs.
/// Sources:
/// - Intel SDM Volume 1, Chapter 21, Section 21.3, Table 21-20
///   (PDF at https://cdrdv2.intel.com/v1/dl/getContent/671436).
/// - AMD64 Architecture Programmer's Manual, Volume 3, Appendix E, section E.3.6
///   (PDF at https://docs.amd.com/api/khub/documents/BqEoC2FadIAgbrUdEzDJIg/content).
#[cfg(target_arch = "x86_64")]
fn cpuid_pku_enabled() -> bool {
    use core::arch::x86_64::{__cpuid, __cpuid_count};

    const PKU: u32 = 1 << 3;
    const OSPKE: u32 = 1 << 4;

    #[allow(
        unused_unsafe,
        reason = "rust is transitioning to `__cpuid` being a safe function"
    )]
    let max_leaf = unsafe { __cpuid(0) }.eax;
    if max_leaf < 0x07 {
        return false;
    }
    #[allow(unused_unsafe, reason = "as above")]
    let ecx = unsafe { __cpuid_count(0x07, 0) }.ecx;
    ecx & (PKU | OSPKE) == PKU | OSPKE
}

#[cfg(not(target_arch = "x86_64"))]
fn cpuid_pku_enabled() -> bool {
    false
}
