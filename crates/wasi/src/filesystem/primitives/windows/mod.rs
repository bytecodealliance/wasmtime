use crate::filesystem::primitives::{Advice, DescriptorFlags};
use std::fs;
use std::io::{self, Write};
use std::os::windows::fs::FileExt;
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle};
use std::path::Path;
use windows_sys::Wdk::Storage::FileSystem::FILE_WRITE_THROUGH;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_APPEND_DATA, FILE_GENERIC_WRITE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FILE_WRITE_DATA, ReOpenFile,
};

mod create_dir_unchecked;
mod create_file_at_w;
mod dir_utils;
mod file_info;
mod file_type;
mod get_path;
mod hard_link_unchecked;
mod metadata;
mod oflags;
mod open_fast;
mod open_options_ext;
mod open_unchecked;
mod read_dir;
mod read_link;
mod read_link_unchecked;
mod remove_dir_unchecked;
mod remove_file_or_symlink;
mod remove_file_unchecked;
mod rename_unchecked;
mod set_times;
mod stat_unchecked;
mod symlink_unchecked;

pub(crate) mod errors;

pub(crate) use create_dir_unchecked::*;
pub(crate) use dir_utils::*;
pub(crate) use file_type::FileType;
pub(crate) use hard_link_unchecked::*;
pub(crate) use metadata::Metadata;
pub(crate) use open_fast::*;
pub(crate) use open_options_ext::*;
pub(crate) use open_unchecked::*;
pub(crate) use read_dir::*;
pub(crate) use read_link::*;
pub(crate) use read_link_unchecked::*;
pub(crate) use remove_dir_unchecked::*;
pub(crate) use remove_file_or_symlink::*;
pub(crate) use remove_file_unchecked::*;
pub(crate) use rename_unchecked::*;
pub(crate) use set_times::*;
pub(crate) use stat_unchecked::*;
pub(crate) use symlink_unchecked::*;

pub(super) use oflags::*;

// On Windows, there is a limit of 63 reparse points on any given path.
// <https://docs.microsoft.com/en-us/windows/win32/fileio/reparse-points>
pub(crate) const MAX_SYMLINK_EXPANSIONS: u8 = 63;

/// Returns an error if `path` contains a `:` anywhere.
///
/// This must be called on every path that is resolved relative to a
/// directory handle before it's handed to the OS.
///
/// On Windows `:` is disallowed in file names, and a `:` is either part of a
/// drive prefix, as in `C:` or `C:foo`, or names an NTFS alternate data stream,
/// as in `file:stream`. Rust's `Path::components` parsing of `C:foo` also
/// depends on where it is in the `Path`, for example it's a drive prefix in
/// `C:foo` but it's a normal component in `./C:foo`. The `primitives` module in
/// this crate generally assumes that a `Normal` path component parsed alone
/// will again yield a `Normal` path component, but Windows drive prefixes break
/// this assumption.
///
/// Given all this the most conservative thing to do is to simply disallow `:`
/// everywhere on Windows. It's already not allowed in filenames and otherwise
/// handling it would require extremely careful management of parsing and
/// ensuring that just because a path component is `Normal` once doesn't mean
/// it'll re-parse as `Normal`.
pub(crate) fn check_no_colon(path: &Path) -> io::Result<()> {
    if path.as_os_str().as_encoded_bytes().contains(&b':') {
        return Err(errors::invalid_name());
    }
    Ok(())
}

/// Queries the synchronization-related status flags of `file`.
///
/// Windows only has `FILE_WRITE_THROUGH`, which is what both `sync` and
/// `dsync` are translated to at open time.
pub(crate) fn status_flags(file: &fs::File) -> io::Result<DescriptorFlags> {
    let mode = file_info::query_mode_information(file.as_handle())?;
    let mut ret = DescriptorFlags::empty();
    ret.set(
        DescriptorFlags::REQUESTED_WRITE_SYNC,
        mode & FILE_WRITE_THROUGH != 0,
    );
    Ok(ret)
}

/// Provides the OS with `advice` about the expected access pattern of the
/// `len` bytes of `file` starting at `offset`.
///
/// This is currently a no-op on Windows.
pub(crate) fn advise(file: &fs::File, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
    let _ = (file, offset, len, advice);
    Ok(())
}

/// Appends `data` to the end of `file`, leaving the file's current position
/// unspecified afterwards.
pub(crate) fn append_cursor_unspecified(file: &fs::File, data: &[u8]) -> io::Result<usize> {
    let file = file.as_handle();
    let access = file_info::query_access_information(file)?;

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

/// Writes `data` to `file` at `pos`, leaving the file's current position
/// unspecified afterwards.
pub(crate) fn write_at_cursor_unspecified(
    file: &fs::File,
    data: &[u8],
    pos: u64,
) -> io::Result<usize> {
    file.seek_write(data, pos)
}

/// Reads from `file` at `pos` into `buf`, leaving the file's current
/// position unspecified afterwards.
pub(crate) fn read_at_cursor_unspecified(
    file: &fs::File,
    buf: &mut [u8],
    pos: u64,
) -> io::Result<usize> {
    file.seek_read(buf, pos)
}

fn reopen_file(
    handle: BorrowedHandle<'_>,
    access_mode: u32,
    share_mode: u32,
    flags: u32,
) -> io::Result<fs::File> {
    let new_handle = unsafe { ReOpenFile(handle.as_raw_handle(), access_mode, share_mode, flags) };

    if new_handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    Ok(unsafe { fs::File::from_raw_handle(new_handle) })
}

/// Open a directory by performing an unsandboxed `openat`-like operation.
#[inline]
pub(super) fn open_dir_unchecked(start: &fs::File, path: &Path) -> io::Result<fs::File> {
    open_unchecked(start, path, &dir_options()).map_err(Into::into)
}
