//!

use super::{pkru, sys};
use anyhow::{Context, Result};
use std::sync::OnceLock;

/// Check if the MPK feature is supported.
pub fn is_supported() -> bool {
    cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") && pkru::has_cpuid_bit_set()
    // TODO: we cannot check CR4 due to privilege
}

/// Allocate all protection keys available to this process.
///
/// This asks the kernel for all available keys (we expect 1-15; 0 is
/// kernel-reserved) in a thread-safe way. This avoids interference when
/// multiple threads try to allocate keys at the same time (e.g., during
/// testing). It also ensures that a single copy of the keys are reserved for
/// the lifetime of the process.
///
/// TODO: this is not the best-possible design. This creates global state that
/// would prevent any other code in the process from using protection keys; the
/// `KEYS` are never deallocated from the system with `pkey_dealloc`.
pub fn keys() -> &'static [ProtectionKey] {
    let keys = KEYS.get_or_init(|| {
        let mut allocated = vec![];
        if is_supported() {
            while let Ok(key_id) = sys::pkey_alloc(0, 0) {
                debug_assert!(key_id < 16);
                // UNSAFETY: here we unsafely assume that the system-allocated pkey
                // will exist forever.
                let pkey = ProtectionKey(key_id);
                debug_assert_eq!(pkey.as_stripe(), allocated.len());
                allocated.push(pkey);
            }
        }
        allocated
    });
    &keys
}
static KEYS: OnceLock<Vec<ProtectionKey>> = OnceLock::new();

/// Only allow access to pages marked by the keys set in `mask`.
///
/// Any accesses to pages marked by another key will result in a `SIGSEGV`
/// fault.
pub fn allow(mask: ProtectionMask) {
    let mut allowed = 0;
    for i in 0..16 {
        if mask.0 & (1 << i) == 1 {
            allowed |= 0b11 << (i * 2);
        }
    }

    let previous = pkru::read();
    pkru::write(pkru::DISABLE_ACCESS ^ allowed);
    log::debug!("PKRU change: {:#034b} => {:#034b}", previous, pkru::read());
}

/// An MPK protection key.
///
/// The expected usage is:
/// - allocate a new key with [`Pkey::new`]
/// - mark some regions of memory as accessible with [`Pkey::protect`]
/// - [`allow`] or disallow access to the memory regions using a
///   [`ProtectionMask`]; any accesses to unmarked pages result in a fault
/// - drop the key
///
/// Since this kernel is allocated from the kernel, we must inform the kernel
/// when it is dropped. Similarly, to retrieve all available protection keys,
/// one must request them from the kernel (e.g., call [`Pkey::new`] until it
/// fails).
///
/// Because MPK may not be available on all systems, [`Pkey`] wraps an `Option`
/// that will always be `None` if MPK is not supported. The idea here is that
/// the API can remain the same regardless of MPK support.
#[derive(Clone, Copy, Debug)]
pub struct ProtectionKey(u32);

impl ProtectionKey {
    /// Mark a page as protected by this [`Pkey`].
    ///
    /// This "colors" the pages of `region` via a kernel `pkey_mprotect` call to
    /// only allow reads and writes when this [`Pkey`] is activated (see
    /// [`Pkey::activate`]).
    ///
    /// # Errors
    ///
    /// This will fail if the region is not page aligned or for some unknown
    /// kernel reason.
    pub fn protect(&self, region: &mut [u8]) -> Result<()> {
        let addr = region.as_mut_ptr() as usize;
        let len = region.len();
        let prot = sys::PROT_READ | sys::PROT_WRITE;
        sys::pkey_mprotect(addr, len, prot, self.0).with_context(|| {
            format!(
                "failed to mark region with pkey (addr = {addr:#x}, len = {len}, prot = {prot:#b})"
            )
        })
    }

    /// Convert the [`Pkey`] to its 0-based index; this is useful for
    /// determining which allocation "stripe" a key belongs to.
    ///
    /// This function assumes that the kernel has allocated key 0 for itself.
    pub fn as_stripe(&self) -> usize {
        debug_assert!(self.0 != 0);
        self.0 as usize - 1
    }
}

/// A bit field indicating which protection keys should be *allowed*.
///
/// When bit `n` is set, it means the protection key is allowed--conversely,
/// protection is disabled for this key.
pub struct ProtectionMask(u16);
impl ProtectionMask {
    /// Allow access from all protection keys.
    pub fn all() -> Self {
        Self(u16::MAX)
    }

    /// Only allow access to memory protected with protection key 0; note that
    /// this does not mean "none" but rather allows access from the default
    /// kernel protection key.
    pub fn zero() -> Self {
        Self(1)
    }

    /// Include `pkey` as another allowed protection key in the mask.
    pub fn or(self, pkey: ProtectionKey) -> Self {
        Self(self.0 | 1 << pkey.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_is_supported() {
        println!("is pku supported = {}", is_supported());
    }

    #[test]
    fn check_initialized_keys() {
        if is_supported() {
            assert!(!keys().is_empty())
        }
    }

    #[test]
    fn check_invalid_mark() {
        let pkey = keys()[0];
        let unaligned_region = unsafe {
            let addr = 1 as *mut u8; // this is not page-aligned!
            let len = 1;
            std::slice::from_raw_parts_mut(addr, len)
        };
        let result = pkey.protect(unaligned_region);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "failed to mark region with pkey (addr = 0x1, len = 1, prot = 0b11)"
        );
    }
}
