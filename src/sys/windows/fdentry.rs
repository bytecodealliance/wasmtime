use crate::host;

use std::fs::File;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
use std::path::PathBuf;
use winapi::shared::minwindef::FALSE;
use winapi::um::handleapi::DuplicateHandle;
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::winnt::DUPLICATE_SAME_ACCESS;

#[derive(Clone, Debug)]
pub struct FdObject {
    pub ty: host::__wasi_filetype_t,
    pub raw_handle: RawHandle,
    pub needs_close: bool,
    // TODO: directories
}

#[derive(Clone, Debug)]
pub struct FdEntry {
    pub fd_object: FdObject,
    pub rights_base: host::__wasi_rights_t,
    pub rights_inheriting: host::__wasi_rights_t,
    pub preopen_path: Option<PathBuf>,
}

impl Drop for FdObject {
    fn drop(&mut self) {
        if self.needs_close {
            unsafe {
                if winapi::um::handleapi::CloseHandle(self.raw_handle) == 0 {
                    // TODO: use DWORD WINAPI GetLastError(void) to get error
                    eprintln!("FdObject::drop(): couldn't close raw Handle");
                }
            }
        }
    }
}

impl FdEntry {
    pub fn from_file(file: File) -> Self {
        unsafe { Self::from_raw_handle(file.into_raw_handle()) }
    }

    pub fn duplicate<F: AsRawHandle>(fd: &F) -> Self {
        unsafe {
            let source = fd.as_raw_handle();
            let mut dest = 0 as RawHandle;

            let cur_proc = GetCurrentProcess();
            if DuplicateHandle(
                cur_proc,
                source,
                cur_proc,
                &mut dest,
                0, // dwDesiredAccess; this flag is ignored if DUPLICATE_SAME_ACCESS is specified
                FALSE,
                DUPLICATE_SAME_ACCESS,
            ) == FALSE
            {
                panic!("Couldn't duplicate handle");
            }

            Self::from_raw_handle(dest)
        }
    }
}

impl FromRawHandle for FdEntry {
    // TODO: implement
    unsafe fn from_raw_handle(raw_handle: RawHandle) -> Self {
        let (ty, rights_base, rights_inheriting) = (
            host::__WASI_FILETYPE_REGULAR_FILE,
            host::RIGHTS_REGULAR_FILE_BASE,
            host::RIGHTS_REGULAR_FILE_INHERITING,
        );

        Self {
            fd_object: FdObject {
                ty,
                raw_handle,
                needs_close: true,
            },
            rights_base,
            rights_inheriting,
            preopen_path: None,
        }
    }
}
