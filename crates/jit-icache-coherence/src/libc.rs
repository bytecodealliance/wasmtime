#![allow(unused)]

use libc::{syscall, EINVAL, EPERM};
use std::ffi::c_void;
use std::io::{Error, Result};

const MEMBARRIER_CMD_GLOBAL: libc::c_int = 1;
const MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE: libc::c_int = 32;
const MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE: libc::c_int = 64;

/// See docs on [crate::pipeline_flush] for a description of what this function is trying to do.
#[inline]
pub(crate) fn pipeline_flush() -> Result<()> {
    // This implementation is not very well commented, but see [rustix::pipeline_flush].
    // We should keep these two implementations in sync, since they are trying to do the
    // exact same thing.

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    match membarrier(MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE) {
        Ok(_) => {}
        Err(e) if e.raw_os_error().unwrap() == EPERM => {
            membarrier(MEMBARRIER_CMD_REGISTER_PRIVATE_EXPEDITED_SYNC_CORE)?;
            membarrier(MEMBARRIER_CMD_PRIVATE_EXPEDITED_SYNC_CORE)?;
        }
        Err(e) if e.raw_os_error().unwrap() == EINVAL => {
            membarrier(MEMBARRIER_CMD_GLOBAL)?;
        }
        e => e?,
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn membarrier(barrier: libc::c_int) -> Result<()> {
    let res = unsafe { syscall(libc::SYS_membarrier, barrier) };
    if res == 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}

/// See docs on [crate::clear_cache] for a description of what this function is trying to do.
#[inline]
pub(crate) fn clear_cache(_ptr: *const c_void, _len: usize) -> Result<()> {
    // TODO: On AArch64 we currently rely on the `mprotect` call that switches the memory from W+R to R+X
    // to do this for us. See [rustix::clear_cache] for more info.

    Ok(())
}
