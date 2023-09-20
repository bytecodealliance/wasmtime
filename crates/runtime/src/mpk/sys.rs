//! Expose the `pkey_*` Linux system calls. See the kernel documentation for
//! more information:
//! - [`pkeys`] overview
//! - [`pkey_alloc`] (with `pkey_free`)
//! - [`pkey_mprotect`]
//! - `pkey_set` is implemented directly in assembly.
//!
//! [`pkey_alloc`]: https://man7.org/linux/man-pages/man2/pkey_alloc.2.html
//! [`pkey_mprotect`]: https://man7.org/linux/man-pages/man2/pkey_mprotect.2.html
//! [`pkeys`]: https://man7.org/linux/man-pages/man7/pkeys.7.html

use crate::page_size;
use anyhow::{anyhow, Result};

/// Protection mask allowing reads of pkey-protected memory (see `prot` in
/// [`pkey_mprotect`]).
pub const PROT_READ: u32 = libc::PROT_READ as u32; // == 0b0001.

/// Protection mask allowing writes of pkey-protected memory (see `prot` in
/// [`pkey_mprotect`]).
pub const PROT_WRITE: u32 = libc::PROT_WRITE as u32; // == 0b0010;

/// Allocate a new protection key in the Linux kernel ([docs]); returns the
/// key ID.
///
/// [docs]: https://man7.org/linux/man-pages/man2/pkey_alloc.2.html
///
/// Each process has its own separate pkey index; e.g., if process `m`
/// allocates key 1, process `n` can as well.
pub fn pkey_alloc(flags: u32, access_rights: u32) -> Result<u32> {
    debug_assert_eq!(flags, 0); // reserved for future use--must be 0.
    let result = unsafe { libc::syscall(libc::SYS_pkey_alloc, flags, access_rights) };
    if result >= 0 {
        Ok(result.try_into().expect("TODO"))
    } else {
        debug_assert_eq!(result, -1); // only this error result is expected.
        Err(anyhow!(unsafe { errno_as_string() }))
    }
}

/// Free a kernel protection key ([docs]).
///
/// [docs]: https://man7.org/linux/man-pages/man2/pkey_alloc.2.html
#[allow(dead_code)]
pub fn pkey_free(key: u32) -> Result<()> {
    let result = unsafe { libc::syscall(libc::SYS_pkey_free, key) };
    if result == 0 {
        Ok(())
    } else {
        debug_assert_eq!(result, -1); // only this error result is expected.
        Err(anyhow!(unsafe { errno_as_string() }))
    }
}

/// Change the access protections for a page-aligned memory region ([docs]).
///
/// [docs]: https://man7.org/linux/man-pages/man2/pkey_mprotect.2.html
pub fn pkey_mprotect(addr: usize, len: usize, prot: u32, key: u32) -> Result<()> {
    let page_size = page_size();
    if addr % page_size != 0 {
        log::warn!(
            "memory must be page-aligned for MPK (addr = {addr:#x}, page size = {page_size}"
        );
    }
    let result = unsafe { libc::syscall(libc::SYS_pkey_mprotect, addr, len, prot, key) };
    if result == 0 {
        Ok(())
    } else {
        debug_assert_eq!(result, -1); // only this error result is expected.
        Err(anyhow!(unsafe { errno_as_string() }))
    }
}

/// Helper function for retrieving the libc error message for the current
/// error (see GNU libc's ["Checking for Errors"] documentation).
///
/// ["Checking for Errors"]: https://www.gnu.org/software/libc/manual/html_node/Checking-for-Errors.html
unsafe fn errno_as_string() -> String {
    let errno = *libc::__errno_location();
    let err_ptr = libc::strerror(errno);
    std::ffi::CStr::from_ptr(err_ptr)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "cannot be run when keys() has already allocated all keys"]
    #[test]
    fn check_allocate_and_free() {
        let key = pkey_alloc(0, 0).unwrap();
        assert_eq!(key, 1);
        // It may seem strange to assert the key ID here, but we already
        // make some assumptions:
        //  1. we are running on Linux with `pku` enabled
        //  2. Linux will allocate key 0 for itself
        //  3. we are running this test in non-MPK mode and no one else is
        //     using pkeys
        // If these assumptions are incorrect, this test can be removed.
        pkey_free(key).unwrap()
    }

    #[test]
    fn check_invalid_free() {
        let result = pkey_free(42);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Invalid argument");
    }

    #[test]
    #[should_panic]
    fn check_invalid_alloc_flags() {
        pkey_alloc(42, 0).unwrap();
    }

    #[test]
    fn check_invalid_alloc_rights() {
        assert!(pkey_alloc(0, 42).is_err());
    }
}
