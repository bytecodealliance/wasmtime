use super::file_serial_no;
use super::oshandle::RawOsHandle;
use crate::path;
use crate::sys::osdir::OsDir;
use crate::sys::osfile::OsFile;
use crate::sys::AsFile;
use crate::wasi::types;
use crate::Result;
use log::trace;
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle};
use std::path::Path;
use winx::file::{AccessMode, FileModeInformation, Flags};

pub(crate) fn fdstat_get(file: &File) -> Result<types::Fdflags> {
    let mut fdflags = types::Fdflags::empty();
    let handle = file.as_raw_handle();
    let access_mode = winx::file::query_access_information(handle)?;
    let mode = winx::file::query_mode_information(handle)?;

    // Append without write implies append-only (__WASI_FDFLAGS_APPEND)
    if access_mode.contains(AccessMode::FILE_APPEND_DATA)
        && !access_mode.contains(AccessMode::FILE_WRITE_DATA)
    {
        fdflags |= types::Fdflags::APPEND;
    }

    if mode.contains(FileModeInformation::FILE_WRITE_THROUGH) {
        // Only report __WASI_FDFLAGS_SYNC
        // This is technically the only one of the O_?SYNC flags Windows supports.
        fdflags |= types::Fdflags::SYNC;
    }

    // Files do not support the `__WASI_FDFLAGS_NONBLOCK` flag

    Ok(fdflags)
}

// TODO Investigate further for Stdio handles. `ReOpenFile` requires the file
// handle came from `CreateFile`, but the Rust's libstd will use `GetStdHandle`
// rather than `CreateFile`. Relevant discussion can be found in:
// https://github.com/rust-lang/rust/issues/40490
pub(crate) fn fdstat_set_flags(
    file: &File,
    fdflags: types::Fdflags,
) -> Result<Option<RawOsHandle>> {
    let handle = file.as_raw_handle();
    let access_mode = winx::file::query_access_information(handle)?;
    let new_access_mode = file_access_mode_from_fdflags(
        fdflags,
        access_mode.contains(AccessMode::FILE_READ_DATA),
        access_mode.contains(AccessMode::FILE_WRITE_DATA)
            | access_mode.contains(AccessMode::FILE_APPEND_DATA),
    );
    unsafe {
        Ok(Some(RawOsHandle::from_raw_handle(winx::file::reopen_file(
            handle,
            new_access_mode,
            fdflags.into(),
        )?)))
    }
}

pub(crate) fn advise(
    _file: &OsFile,
    _advice: types::Advice,
    _offset: types::Filesize,
    _len: types::Filesize,
) -> Result<()> {
    Ok(())
}

fn file_access_mode_from_fdflags(fdflags: types::Fdflags, read: bool, write: bool) -> AccessMode {
    let mut access_mode = AccessMode::READ_CONTROL;

    // Note that `GENERIC_READ` and `GENERIC_WRITE` cannot be used to properly support append-only mode
    // The file-specific flags `FILE_GENERIC_READ` and `FILE_GENERIC_WRITE` are used here instead
    // These flags have the same semantic meaning for file objects, but allow removal of specific permissions (see below)
    if read {
        access_mode.insert(AccessMode::FILE_GENERIC_READ);
    }

    if write {
        access_mode.insert(AccessMode::FILE_GENERIC_WRITE);
    }

    // For append, grant the handle FILE_APPEND_DATA access but *not* FILE_WRITE_DATA.
    // This makes the handle "append only".
    // Changes to the file pointer will be ignored (like POSIX's O_APPEND behavior).
    if fdflags.contains(&types::Fdflags::APPEND) {
        access_mode.insert(AccessMode::FILE_APPEND_DATA);
        access_mode.remove(AccessMode::FILE_WRITE_DATA);
    }

    access_mode
}

// On Windows there is apparently no support for seeking the directory stream in the OS.
// cf. https://github.com/WebAssembly/WASI/issues/61
//
// The implementation here may perform in O(n^2) if the host buffer is O(1)
// and the number of directory entries is O(n).
// TODO: Add a heuristic optimization to achieve O(n) time in the most common case
//      where fd_readdir is resumed where it previously finished
//
// Correctness of this approach relies upon one assumption: that the order of entries
// returned by `FindNextFileW` is stable, i.e. doesn't change if the directory
// contents stay the same. This invariant is crucial to be able to implement
// any kind of seeking whatsoever without having to read the whole directory at once
// and then return the data from cache. (which leaks memory)
//
// The MSDN documentation explicitly says that the order in which the search returns the files
// is not guaranteed, and is dependent on the file system.
// cf. https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-findnextfilew
//
// This stackoverflow post suggests that `FindNextFileW` is indeed stable and that
// the order of directory entries depends **only** on the filesystem used, but the
// MSDN documentation is not clear about this.
// cf. https://stackoverflow.com/questions/47380739/is-findfirstfile-and-findnextfile-order-random-even-for-dvd
//
// Implementation details:
// Cookies for the directory entries start from 1. (0 is reserved by wasi::__WASI_DIRCOOKIE_START)
// .        gets cookie = 1
// ..       gets cookie = 2
// other entries, in order they were returned by FindNextFileW get subsequent integers as their cookies
pub(crate) fn readdir(
    dirfd: &OsDir,
    cookie: types::Dircookie,
) -> Result<Box<dyn Iterator<Item = Result<(types::Dirent, String)>>>> {
    use winx::file::get_file_path;

    let cookie = cookie.try_into()?;
    let path = get_file_path(&*dirfd.as_file()?)?;
    // std::fs::ReadDir doesn't return . and .., so we need to emulate it
    let path = Path::new(&path);
    // The directory /.. is the same as / on Unix (at least on ext4), so emulate this behavior too
    let parent = path.parent().unwrap_or(path);
    let dot = dirent_from_path(path, ".", 1)?;
    let dotdot = dirent_from_path(parent, "..", 2)?;

    trace!("    | fd_readdir impl: executing std::fs::ReadDir");
    let iter = path.read_dir()?.zip(3..).map(|(dir, no)| {
        let dir: std::fs::DirEntry = dir?;
        let ftype = dir.file_type()?;
        let name = path::from_host(dir.file_name())?;
        let d_ino = File::open(dir.path()).and_then(|f| file_serial_no(&f))?;
        let dirent = types::Dirent {
            d_namlen: name.len().try_into()?,
            d_type: ftype.into(),
            d_ino,
            d_next: no,
        };

        Ok((dirent, name))
    });

    // into_iter for arrays is broken and returns references instead of values,
    // so we need to use vec![...] and do heap allocation
    // See https://github.com/rust-lang/rust/issues/25725
    let iter = vec![dot, dotdot].into_iter().map(Ok).chain(iter);

    // Emulate seekdir(). This may give O(n^2) complexity if used with a
    // small host_buf, but this is difficult to implement efficiently.
    //
    // See https://github.com/WebAssembly/WASI/issues/61 for more details.
    Ok(Box::new(iter.skip(cookie)))
}

fn dirent_from_path<P: AsRef<Path>>(
    path: P,
    name: &str,
    cookie: types::Dircookie,
) -> Result<(types::Dirent, String)> {
    let path = path.as_ref();
    trace!("dirent_from_path: opening {}", path.to_string_lossy());

    // To open a directory on Windows, FILE_FLAG_BACKUP_SEMANTICS flag must be used
    let file = OpenOptions::new()
        .custom_flags(Flags::FILE_FLAG_BACKUP_SEMANTICS.bits())
        .read(true)
        .open(path)?;
    let ty = file.metadata()?.file_type();
    let name = name.to_owned();
    let dirent = types::Dirent {
        d_namlen: name.len().try_into()?,
        d_next: cookie,
        d_type: ty.into(),
        d_ino: file_serial_no(&file)?,
    };
    Ok((dirent, name))
}

pub(crate) fn filestat_get(file: &File) -> Result<types::Filestat> {
    let filestat = file.try_into()?;
    Ok(filestat)
}
