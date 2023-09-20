//! Memory Protection Keys (MPK) implementation for use in striped memory
//! allocation.
//!
//! MPK is an x86 feature available on relatively recent versions of Intel and
//! AMD CPUs. In Linux, this feature is named `pku` (protection keys userspace)
//! and consists of three new system calls: `pkey_alloc`, `pkey_free`, and
//! `pkey_mprotect` (see the [Linux documentation]). This crate provides an
//! abstraction, [`Pkey`], that the [pooling allocator] applies to contiguous
//! memory allocations, allowing it to avoid guard pages in some cases and more
//! efficiently use memory. This technique was first presented in a 2022 paper:
//! [Segue and ColorGuard: Optimizing SFI Performance and Scalability on Modern
//! x86][colorguard].
//!
//! [pooling allocator]: crate::PoolingInstanceAllocator
//! [Linux documentation]:
//!     https://www.kernel.org/doc/html/latest/core-api/protection-keys.html
//! [colorguard]: https://plas2022.github.io/files/pdf/SegueColorGuard.pdf
//!
//! On x86_64 Linux systems, this module implements the various parts necessary
//! to use MPK in Wasmtime:
//! - [`is_supported`] indicates whether the feature is available at runtime
//! - [`Pkey`] provides safe access to the kernel-allocated protection keys
//! - the `sys` module bridges the gap to Linux's `pkey_*` system calls
//! - the `pkru` module controls the x86 `PKRU` register (and other CPU state)
//!
//! On any other kind of machine, this module exposes noop implementations of
//! the public interface.

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod enabled;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod pkru;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod sys;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub use enabled::{allow, is_supported, keys, ProtectionKey, ProtectionMask};

#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
mod disabled;
#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
pub use disabled::{allow, is_supported, keys, ProtectionKey, ProtectionMask};
