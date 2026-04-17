use crate::filesystem::{Advice, DescriptorFlags};
use io_lifetimes::AsFilelike;
use std::fs::File;
use std::io::{self, Write};
use std::mem::{self, MaybeUninit};
use std::os::windows::fs::FileExt;
use std::os::windows::io::*;
use windows_sys::Wdk::Storage::FileSystem::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::IO::*;

pub(crate) fn get_flags(file: impl AsHandle) -> io::Result<DescriptorFlags> {
    let file = file.as_handle();
    let mode = query_mode_information(file)?;
    let mut ret = DescriptorFlags::empty();
    ret.set(
        DescriptorFlags::REQUESTED_WRITE_SYNC,
        mode & FILE_WRITE_THROUGH != 0,
    );
    Ok(ret)
}

pub(crate) fn advise(file: impl AsHandle, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
    let _ = (file, offset, len, advice);

    // ... noop for now ...

    Ok(())
}

pub(crate) fn append_cursor_unspecified(file: impl AsHandle, data: &[u8]) -> io::Result<usize> {
    let file = file.as_handle();
    let access = query_access_information(file)?;

    // If this file doesn't allow writing then it can't be appended to.
    if access & (FILE_WRITE_DATA | FILE_APPEND_DATA) == 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "file not opened with write or append access",
        ));
    }

    // Reopen the file with append
    reopen_file(
        file,
        FILE_GENERIC_WRITE & !FILE_WRITE_DATA,
        // Files on Windows are opened with DELETE, READ, and WRITE share mode
        // by default (see OpenOptions in stdlib) This keeps the same share mode
        // when reopening the file handle
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        0,
    )?
    .write(data)
}

pub(crate) fn write_at_cursor_unspecified(
    file: impl AsHandle,
    data: &[u8],
    pos: u64,
) -> io::Result<usize> {
    file.as_filelike_view::<File>().seek_write(data, pos)
}

pub(crate) fn read_at_cursor_unspecified(
    file: impl AsHandle,
    buf: &mut [u8],
    pos: u64,
) -> io::Result<usize> {
    file.as_filelike_view::<File>().seek_read(buf, pos)
}

fn query_access_information(handle: BorrowedHandle<'_>) -> io::Result<u32> {
    unsafe {
        Ok(
            nt_query_information_file::<FILE_ACCESS_INFORMATION>(handle, FileAccessInformation)?
                .AccessFlags,
        )
    }
}

fn reopen_file(
    handle: BorrowedHandle<'_>,
    access_mode: u32,
    share_mode: u32,
    flags: u32,
) -> io::Result<File> {
    let new_handle = unsafe { ReOpenFile(handle.as_raw_handle(), access_mode, share_mode, flags) };

    if new_handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { File::from_raw_handle(new_handle) })
}

fn query_mode_information(handle: BorrowedHandle<'_>) -> io::Result<u32> {
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
