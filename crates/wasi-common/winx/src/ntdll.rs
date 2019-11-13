//! Module for importing functions from ntdll.dll.
//! The winapi crate does not expose these Windows API functions.

#![allow(nonstandard_style)]

use std::ffi::c_void;
use std::os::raw::c_ulong;
use std::os::windows::prelude::RawHandle;
use winapi::shared::ntdef::NTSTATUS;
use winapi::um::winnt::ACCESS_MASK;

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) enum FILE_INFORMATION_CLASS {
    FileAccessInformation = 8,
    FileModeInformation = 16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) union IO_STATUS_BLOCK_u {
    pub Status: NTSTATUS,
    pub Pointer: *mut c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct IO_STATUS_BLOCK {
    pub u: IO_STATUS_BLOCK_u,
    pub Information: *mut c_void,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub(crate) struct FILE_ACCESS_INFORMATION {
    pub AccessFlags: ACCESS_MASK,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub(crate) struct FILE_MODE_INFORMATION {
    pub Mode: c_ulong,
}

impl Default for IO_STATUS_BLOCK {
    #[inline]
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[link(name = "ntdll")]
extern "C" {
    // https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryinformationfile
    pub(crate) fn NtQueryInformationFile(
        FileHandle: RawHandle,
        IoStatusBlock: *mut IO_STATUS_BLOCK,
        FileInformation: *mut c_void,
        Length: c_ulong,
        FileInformationClass: FILE_INFORMATION_CLASS,
    ) -> NTSTATUS;

    // https://docs.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-rtlntstatustodoserror
    pub(crate) fn RtlNtStatusToDosError(status: NTSTATUS) -> c_ulong;
}
