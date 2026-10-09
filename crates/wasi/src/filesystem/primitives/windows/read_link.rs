use crate::filesystem::primitives::{FollowSymlinks, OpenOptions, OpenOptionsExt, open};
use std::ffi::OsString;
use std::mem::{MaybeUninit, offset_of};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::{fs, io, ptr, slice};
use windows_sys::Wdk::Storage::FileSystem::{
    REPARSE_DATA_BUFFER, REPARSE_DATA_BUFFER_0_0, REPARSE_DATA_BUFFER_0_1, SYMLINK_FLAG_RELATIVE,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, MAXIMUM_REPARSE_DATA_BUFFER_SIZE,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::FSCTL_GET_REPARSE_POINT;
use windows_sys::Win32::System::SystemServices::{
    IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK,
};

/// Perform a `readlinkat`-like operation, ensuring that the resolution of the
/// path never escapes the directory tree rooted at `start`.
///
/// The link itself is opened with the sandboxed `open`, without following it,
/// and its target is read from the reparse point data of the opened handle.
/// Whether the target is absolute is not checked here; that's left to the
/// caller.
pub(crate) fn read_link(start: &fs::File, path: &Path) -> io::Result<PathBuf> {
    // Open the link with no access mode, instead of generic read.
    // By default FILE_LIST_DIRECTORY is denied for the junction "C:\Documents and
    // Settings", so this is needed for a common case.
    let mut opts = OpenOptions::new();
    opts.access_mode(0);
    opts.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    opts.follow(FollowSymlinks::No);
    let file = open(start, path, &opts)?;
    read_reparse_point(&file)
}

/// Reads the target of the symlink or junction that `file` is a handle to.
fn read_reparse_point(file: &fs::File) -> io::Result<PathBuf> {
    #[repr(C, align(8))]
    struct Buf([MaybeUninit<u8>; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize]);

    let mut space = Buf([MaybeUninit::uninit(); MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize]);
    let mut bytes = 0;
    let ok = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            FSCTL_GET_REPARSE_POINT,
            ptr::null(),
            0,
            space.0.as_mut_ptr().cast(),
            space.0.len() as u32,
            &mut bytes,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let bytes = bytes as usize;
    let buf = space.0.as_ptr().cast::<REPARSE_DATA_BUFFER>();
    let data = offset_of!(REPARSE_DATA_BUFFER, Anonymous);

    // SAFETY: each field read below is first checked to be within the `bytes`
    // that the kernel initialized, and `Buf` is suitably aligned for
    // `REPARSE_DATA_BUFFER`.
    unsafe {
        if bytes < data {
            return Err(invalid_reparse_data());
        }
        let (subst_off, subst_len, path_buffer, relative) = match (*buf).ReparseTag {
            IO_REPARSE_TAG_SYMLINK => {
                let path_buffer = data + offset_of!(REPARSE_DATA_BUFFER_0_0, PathBuffer);
                if bytes < path_buffer {
                    return Err(invalid_reparse_data());
                }
                let info = &(*buf).Anonymous.SymbolicLinkReparseBuffer;
                (
                    info.SubstituteNameOffset,
                    info.SubstituteNameLength,
                    path_buffer,
                    info.Flags & SYMLINK_FLAG_RELATIVE != 0,
                )
            }
            IO_REPARSE_TAG_MOUNT_POINT => {
                let path_buffer = data + offset_of!(REPARSE_DATA_BUFFER_0_1, PathBuffer);
                if bytes < path_buffer {
                    return Err(invalid_reparse_data());
                }
                let info = &(*buf).Anonymous.MountPointReparseBuffer;
                (
                    info.SubstituteNameOffset,
                    info.SubstituteNameLength,
                    path_buffer,
                    false,
                )
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Unsupported reparse point type",
                ));
            }
        };

        let start = path_buffer + usize::from(subst_off);
        let end = start + usize::from(subst_len);
        if end > bytes || start % 2 != 0 || end % 2 != 0 {
            return Err(invalid_reparse_data());
        }
        let base = space.0.as_ptr().cast::<u8>();
        let mut subst = slice::from_raw_parts(base.add(start).cast::<u16>(), (end - start) / 2);

        // Absolute paths start with an NT internal namespace prefix `\??\`.
        // We should not let it leak through.
        if !relative && subst.starts_with(&['\\' as u16, '?' as u16, '?' as u16, '\\' as u16]) {
            subst = &subst[4..];
        }
        Ok(PathBuf::from(OsString::from_wide(subst)))
    }
}

fn invalid_reparse_data() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "malformed reparse point data")
}
