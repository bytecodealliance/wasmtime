use std::io;
use std::mem::{self, MaybeUninit};
use std::os::windows::io::{AsRawHandle, BorrowedHandle};
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_ACCESS_INFORMATION, FILE_INFORMATION_CLASS, FILE_MODE_INFORMATION, FileAccessInformation,
    FileModeInformation, NtQueryInformationFile,
};
use windows_sys::Win32::Foundation::{RtlNtStatusToDosError, STATUS_SUCCESS};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

/// Returns the access rights that `handle` was opened with.
pub(super) fn query_access_information(handle: BorrowedHandle<'_>) -> io::Result<u32> {
    unsafe {
        Ok(
            nt_query_information_file::<FILE_ACCESS_INFORMATION>(handle, FileAccessInformation)?
                .AccessFlags,
        )
    }
}

/// Returns the mode flags (e.g. `FILE_WRITE_THROUGH`) that `handle` was
/// opened with.
pub(super) fn query_mode_information(handle: BorrowedHandle<'_>) -> io::Result<u32> {
    unsafe {
        Ok(nt_query_information_file::<FILE_MODE_INFORMATION>(handle, FileModeInformation)?.Mode)
    }
}

unsafe fn nt_query_information_file<T>(
    handle: BorrowedHandle<'_>,
    info: FILE_INFORMATION_CLASS,
) -> io::Result<T> {
    unsafe {
        let mut io_status_block = mem::zeroed::<IO_STATUS_BLOCK>();
        let mut payload = MaybeUninit::<T>::uninit();

        let status = NtQueryInformationFile(
            handle.as_raw_handle(),
            &mut io_status_block,
            payload.as_mut_ptr().cast(),
            mem::size_of_val(&payload).try_into().unwrap(),
            info,
        );

        if status != STATUS_SUCCESS {
            return Err(io::Error::from_raw_os_error(
                RtlNtStatusToDosError(status) as i32
            ));
        }

        Ok(payload.assume_init())
    }
}
