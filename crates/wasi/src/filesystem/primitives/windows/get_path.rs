use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::{fs, io};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;

/// Maximum total path length for Unicode in Windows.
/// [Maximum path length limitation]: https://docs.microsoft.com/en-us/windows/desktop/FileIO/naming-a-file#maximum-path-length-limitation
const WIDE_MAX_PATH: u32 = 0x7fff;

/// Calculates system path of `file`.
///
/// This function will automatically strip the extended prefix from the
/// resultant path to allow for joining this resultant path with relative
/// components.
pub(crate) fn get_path(file: &fs::File) -> io::Result<PathBuf> {
    // get system path to the handle
    let path = get_final_path_name_by_handle(file)?;

    // strip extended prefix; otherwise we will error out on any relative
    // components with `out_path`
    let wide: Vec<_> = path.as_os_str().encode_wide().collect();
    let wide_final = if wide.starts_with(&['\\' as u16, '\\' as _, '?' as _, '\\' as _]) {
        &wide[4..]
    } else {
        &wide
    };
    Ok(PathBuf::from(OsString::from_wide(wide_final)))
}

/// Convenience function for calling `get_path` and concatenating the result
/// with `path`.
pub(super) fn concatenate(file: &fs::File, path: &Path) -> io::Result<PathBuf> {
    let file_path = get_path(file)?;
    Ok(file_path.join(path))
}

fn get_final_path_name_by_handle(file: &fs::File) -> io::Result<PathBuf> {
    let mut raw_path = vec![0u16; WIDE_MAX_PATH as usize];

    let handle = file.as_raw_handle();
    let res = unsafe {
        GetFinalPathNameByHandleW(handle, raw_path.as_mut_ptr(), raw_path.capacity() as u32, 0)
    };
    if res == 0 {
        return Err(io::Error::last_os_error());
    }

    // obtain a slice containing the written bytes, and check for it being too long
    // (practically probably impossible)
    let written_bytes = raw_path
        .get(..res as usize)
        .ok_or(io::Error::from_raw_os_error(ERROR_BUFFER_OVERFLOW as i32))?;

    Ok(PathBuf::from(OsString::from_wide(written_bytes)))
}
