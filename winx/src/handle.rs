#![allow(non_camel_case_types)]
use crate::{winerror, Result};
use std::os::windows::prelude::RawHandle;
use winapi::shared::minwindef::FALSE;

pub fn dup(old_handle: RawHandle) -> Result<RawHandle> {
    use winapi::um::handleapi::DuplicateHandle;
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::winnt::DUPLICATE_SAME_ACCESS;
    unsafe {
        let mut new_handle = 0 as RawHandle;
        let cur_proc = GetCurrentProcess();
        if DuplicateHandle(
            cur_proc,
            old_handle,
            cur_proc,
            &mut new_handle,
            0, // dwDesiredAccess; this flag is ignored if DUPLICATE_SAME_ACCESS is specified
            FALSE,
            DUPLICATE_SAME_ACCESS,
        ) == FALSE
        {
            Err(winerror::WinError::last())
        } else {
            Ok(new_handle)
        }
    }
}

pub fn close(handle: RawHandle) -> Result<()> {
    use winapi::um::handleapi::CloseHandle;
    if unsafe { CloseHandle(handle) } == FALSE {
        Err(winerror::WinError::last())
    } else {
        Ok(())
    }
}
