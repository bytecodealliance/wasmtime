use std::ffi::c_void;
use std::io::{Error, Result};
use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Threading::FlushProcessWriteBuffers;
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// See docs on [crate::pipeline_flush] for a description of what this function is trying to do.
#[inline]
pub(crate) fn pipeline_flush() -> Result<()> {
    // See: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-flushprocesswritebuffers
    if cfg!(target_arch = "aarch64") {
        unsafe {
            FlushProcessWriteBuffers();
        }
    }

    Ok(())
}

/// See docs on [crate::clear_cache] for a description of what this function is trying to do.
#[inline]
pub(crate) fn clear_cache(ptr: *const c_void, len: usize) -> Result<()> {
    // See:
    //   * https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-flushinstructioncache
    //   * https://devblogs.microsoft.com/oldnewthing/20190902-00/?p=102828
    if cfg!(target_arch = "aarch64") {
        unsafe {
            let res = FlushInstructionCache(GetCurrentProcess(), ptr, len);
            if res == 0 {
                return Err(Error::last_os_error());
            }
        }
    }

    Ok(())
}
