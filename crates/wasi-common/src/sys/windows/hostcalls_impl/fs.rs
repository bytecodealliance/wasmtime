#![allow(non_camel_case_types)]
#![allow(unused)]
use super::fs_helpers::*;
use crate::ctx::WasiCtx;
use crate::fdentry::FdEntry;
use crate::helpers::systemtime_to_timestamp;
use crate::hostcalls_impl::{fd_filestat_set_times_impl, Dirent, FileType, PathGet};
use crate::sys::fdentry_impl::determine_type_rights;
use crate::sys::host_impl::{self, path_from_host};
use crate::sys::hostcalls_impl::fs_helpers::PathGetExt;
use crate::{wasi, Error, Result};
use log::{debug, trace};
use std::convert::TryInto;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Seek, SeekFrom};
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::os::windows::prelude::{AsRawHandle, FromRawHandle};
use std::path::{Path, PathBuf};
use winx::file::{AccessMode, CreationDisposition, FileModeInformation, Flags};

fn read_at(mut file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    // get current cursor position
    let cur_pos = file.seek(SeekFrom::Current(0))?;
    // perform a seek read by a specified offset
    let nread = file.seek_read(buf, offset)?;
    // rewind the cursor back to the original position
    file.seek(SeekFrom::Start(cur_pos))?;
    Ok(nread)
}

fn write_at(mut file: &File, buf: &[u8], offset: u64) -> io::Result<usize> {
    // get current cursor position
    let cur_pos = file.seek(SeekFrom::Current(0))?;
    // perform a seek write by a specified offset
    let nwritten = file.seek_write(buf, offset)?;
    // rewind the cursor back to the original position
    file.seek(SeekFrom::Start(cur_pos))?;
    Ok(nwritten)
}

// TODO refactor common code with unix
pub(crate) fn fd_pread(
    file: &File,
    buf: &mut [u8],
    offset: wasi::__wasi_filesize_t,
) -> Result<usize> {
    read_at(file, buf, offset).map_err(Into::into)
}

// TODO refactor common code with unix
pub(crate) fn fd_pwrite(file: &File, buf: &[u8], offset: wasi::__wasi_filesize_t) -> Result<usize> {
    write_at(file, buf, offset).map_err(Into::into)
}

pub(crate) fn fd_fdstat_get(fd: &File) -> Result<wasi::__wasi_fdflags_t> {
    let mut fdflags = 0;

    let handle = unsafe { fd.as_raw_handle() };

    let access_mode = winx::file::query_access_information(handle)?;
    let mode = winx::file::query_mode_information(handle)?;

    // Append without write implies append-only (__WASI_FDFLAGS_APPEND)
    if access_mode.contains(AccessMode::FILE_APPEND_DATA)
        && !access_mode.contains(AccessMode::FILE_WRITE_DATA)
    {
        fdflags |= wasi::__WASI_FDFLAGS_APPEND;
    }

    if mode.contains(FileModeInformation::FILE_WRITE_THROUGH) {
        // Only report __WASI_FDFLAGS_SYNC
        // This is technically the only one of the O_?SYNC flags Windows supports.
        fdflags |= wasi::__WASI_FDFLAGS_SYNC;
    }

    // Files do not support the `__WASI_FDFLAGS_NONBLOCK` flag

    Ok(fdflags)
}

pub(crate) fn fd_fdstat_set_flags(fd: &File, fdflags: wasi::__wasi_fdflags_t) -> Result<()> {
    unimplemented!("fd_fdstat_set_flags")
}

pub(crate) fn fd_advise(
    _file: &File,
    advice: wasi::__wasi_advice_t,
    _offset: wasi::__wasi_filesize_t,
    _len: wasi::__wasi_filesize_t,
) -> Result<()> {
    match advice {
        wasi::__WASI_ADVICE_DONTNEED
        | wasi::__WASI_ADVICE_SEQUENTIAL
        | wasi::__WASI_ADVICE_WILLNEED
        | wasi::__WASI_ADVICE_NOREUSE
        | wasi::__WASI_ADVICE_RANDOM
        | wasi::__WASI_ADVICE_NORMAL => {}
        _ => return Err(Error::EINVAL),
    }

    Ok(())
}

pub(crate) fn path_create_directory(resolved: PathGet) -> Result<()> {
    let path = resolved.concatenate()?;
    std::fs::create_dir(&path).map_err(Into::into)
}

pub(crate) fn path_link(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    unimplemented!("path_link")
}

pub(crate) fn path_open(
    resolved: PathGet,
    read: bool,
    write: bool,
    oflags: wasi::__wasi_oflags_t,
    fdflags: wasi::__wasi_fdflags_t,
) -> Result<File> {
    use winx::file::{AccessMode, CreationDisposition, Flags};

    // convert open flags
    // note: the calls to `write(true)` are to bypass an internal OpenOption check
    // the write flag will ultimately be ignored when `access_mode` is called below.
    let mut opts = OpenOptions::new();
    match creation_disposition_from_oflags(oflags) {
        CreationDisposition::CREATE_ALWAYS => {
            opts.create(true).write(true);
        }
        CreationDisposition::CREATE_NEW => {
            opts.create_new(true).write(true);
        }
        CreationDisposition::TRUNCATE_EXISTING => {
            opts.truncate(true).write(true);
        }
        _ => {}
    }

    let path = resolved.concatenate()?;

    match path.symlink_metadata().map(|metadata| metadata.file_type()) {
        Ok(file_type) => {
            // check if we are trying to open a symlink
            if file_type.is_symlink() {
                return Err(Error::ELOOP);
            }
            // check if we are trying to open a file as a dir
            if file_type.is_file() && oflags & wasi::__WASI_OFLAGS_DIRECTORY != 0 {
                return Err(Error::ENOTDIR);
            }
        }
        Err(e) => match e.raw_os_error() {
            Some(e) => {
                use winx::winerror::WinError;
                log::debug!("path_open at symlink_metadata error code={:?}", e);
                let e = WinError::from_u32(e as u32);

                if e != WinError::ERROR_FILE_NOT_FOUND {
                    return Err(e.into());
                }
                // file not found, let it proceed to actually
                // trying to open it
            }
            None => {
                log::debug!("Inconvertible OS error: {}", e);
                return Err(Error::EIO);
            }
        },
    }

    opts.access_mode(file_access_mode_from_fdflags(fdflags, read, write).bits())
        .custom_flags(file_flags_from_fdflags(fdflags).bits())
        .open(&path)
        .map_err(Into::into)
}

fn creation_disposition_from_oflags(oflags: wasi::__wasi_oflags_t) -> CreationDisposition {
    if oflags & wasi::__WASI_OFLAGS_CREAT != 0 {
        if oflags & wasi::__WASI_OFLAGS_EXCL != 0 {
            CreationDisposition::CREATE_NEW
        } else {
            CreationDisposition::CREATE_ALWAYS
        }
    } else if oflags & wasi::__WASI_OFLAGS_TRUNC != 0 {
        CreationDisposition::TRUNCATE_EXISTING
    } else {
        CreationDisposition::OPEN_EXISTING
    }
}

fn file_access_mode_from_fdflags(
    fdflags: wasi::__wasi_fdflags_t,
    read: bool,
    write: bool,
) -> AccessMode {
    let mut access_mode = AccessMode::READ_CONTROL;

    if read {
        access_mode.insert(AccessMode::GENERIC_READ);
    }

    if write {
        access_mode.insert(AccessMode::GENERIC_WRITE);
    }

    // For append, grant the handle FILE_APPEND_DATA access but *not* FILE_WRITE_DATA.
    // This makes the handle "append only".
    // Changes to the file pointer will be ignored (like POSIX's O_APPEND behavior).
    if fdflags & wasi::__WASI_FDFLAGS_APPEND != 0 {
        access_mode.insert(AccessMode::FILE_APPEND_DATA);
        access_mode.remove(AccessMode::FILE_WRITE_DATA);
    }

    access_mode
}

fn file_flags_from_fdflags(fdflags: wasi::__wasi_fdflags_t) -> Flags {
    // Enable backup semantics so directories can be opened as files
    let mut flags = Flags::FILE_FLAG_BACKUP_SEMANTICS;

    // Note: __WASI_FDFLAGS_NONBLOCK is purposely being ignored for files
    // While Windows does inherently support a non-blocking mode on files, the WASI API will
    // treat I/O operations on files as synchronous. WASI might have an async-io API in the future.

    // Technically, Windows only supports __WASI_FDFLAGS_SYNC, but treat all the flags as the same.
    if fdflags & wasi::__WASI_FDFLAGS_DSYNC != 0
        || fdflags & wasi::__WASI_FDFLAGS_RSYNC != 0
        || fdflags & wasi::__WASI_FDFLAGS_SYNC != 0
    {
        flags.insert(Flags::FILE_FLAG_WRITE_THROUGH);
    }

    flags
}

fn dirent_from_path<P: AsRef<Path>>(
    path: P,
    name: &str,
    cookie: wasi::__wasi_dircookie_t,
) -> Result<Dirent> {
    let path = path.as_ref();
    trace!("dirent_from_path: opening {}", path.to_string_lossy());

    // To open a directory on Windows, FILE_FLAG_BACKUP_SEMANTICS flag must be used
    let file = OpenOptions::new()
        .custom_flags(Flags::FILE_FLAG_BACKUP_SEMANTICS.bits())
        .read(true)
        .open(path)?;
    let ty = file.metadata()?.file_type();
    Ok(Dirent {
        ftype: filetype_from_std(&ty),
        name: name.to_owned(),
        cookie,
        ino: file_serial_no(&file)?,
    })
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
pub(crate) fn fd_readdir(
    fd: &File,
    cookie: wasi::__wasi_dircookie_t,
) -> Result<impl Iterator<Item = Result<Dirent>>> {
    use winx::file::get_file_path;

    let cookie = cookie.try_into()?;
    let path = get_file_path(fd)?;
    // std::fs::ReadDir doesn't return . and .., so we need to emulate it
    let path = Path::new(&path);
    // The directory /.. is the same as / on Unix (at least on ext4), so emulate this behavior too
    let parent = path.parent().unwrap_or(path);
    let dot = dirent_from_path(path, ".", 1)?;
    let dotdot = dirent_from_path(parent, "..", 2)?;

    trace!("    | fd_readdir impl: executing std::fs::ReadDir");
    let iter = path.read_dir()?.zip(3..).map(|(dir, no)| {
        let dir: std::fs::DirEntry = dir?;

        Ok(Dirent {
            name: path_from_host(dir.file_name())?,
            ftype: filetype_from_std(&dir.file_type()?),
            ino: File::open(dir.path()).and_then(|f| file_serial_no(&f))?,
            cookie: no,
        })
    });

    // into_iter for arrays is broken and returns references instead of values,
    // so we need to use vec![...] and do heap allocation
    // See https://github.com/rust-lang/rust/issues/25725
    let iter = vec![dot, dotdot].into_iter().map(Ok).chain(iter);

    // Emulate seekdir(). This may give O(n^2) complexity if used with a
    // small host_buf, but this is difficult to implement efficiently.
    //
    // See https://github.com/WebAssembly/WASI/issues/61 for more details.
    Ok(iter.skip(cookie))
}

pub(crate) fn path_readlink(resolved: PathGet, buf: &mut [u8]) -> Result<usize> {
    use winx::file::get_file_path;

    let path = resolved.concatenate()?;
    let target_path = path.read_link()?;

    // since on Windows we are effectively emulating 'at' syscalls
    // we need to strip the prefix from the absolute path
    // as otherwise we will error out since WASI is not capable
    // of dealing with absolute paths
    let dir_path = get_file_path(resolved.dirfd())?;
    let dir_path = PathBuf::from(strip_extended_prefix(dir_path));
    let target_path = target_path
        .strip_prefix(dir_path)
        .map_err(|_| Error::ENOTCAPABLE)
        .and_then(|path| path.to_str().map(String::from).ok_or(Error::EILSEQ))?;

    if buf.len() > 0 {
        let mut chars = target_path.chars();
        let mut nread = 0usize;

        for i in 0..buf.len() {
            match chars.next() {
                Some(ch) => {
                    buf[i] = ch as u8;
                    nread += 1;
                }
                None => break,
            }
        }

        Ok(nread)
    } else {
        Ok(0)
    }
}

fn strip_trailing_slashes_and_concatenate(resolved: &PathGet) -> Result<Option<PathBuf>> {
    if resolved.path().ends_with('/') {
        let suffix = resolved.path().trim_end_matches('/');
        concatenate(resolved.dirfd(), Path::new(suffix)).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use std::fs;

    let old_path = resolved_old.concatenate()?;
    let new_path = resolved_new.concatenate()?;

    // First sanity check: check we're not trying to rename dir to file or vice versa.
    // NB on Windows, the former is actually permitted [std::fs::rename].
    //
    // [std::fs::rename]: https://doc.rust-lang.org/std/fs/fn.rename.html
    if old_path.is_dir() && new_path.is_file() {
        return Err(Error::ENOTDIR);
    }
    // Second sanity check: check we're not trying to rename a file into a path
    // ending in a trailing slash.
    if old_path.is_file() && resolved_new.path().ends_with('/') {
        return Err(Error::ENOTDIR);
    }

    // TODO handle symlinks

    fs::rename(&old_path, &new_path).or_else(|e| match e.raw_os_error() {
        Some(e) => {
            use winx::winerror::WinError;

            log::debug!("path_rename at rename error code={:?}", e);
            match WinError::from_u32(e as u32) {
                WinError::ERROR_ACCESS_DENIED => {
                    // So most likely dealing with new_path == dir.
                    // Eliminate case old_path == file first.
                    if old_path.is_file() {
                        Err(Error::EISDIR)
                    } else {
                        // Ok, let's try removing an empty dir at new_path if it exists
                        // and is a nonempty dir.
                        fs::remove_dir(&new_path)
                            .and_then(|()| fs::rename(old_path, new_path))
                            .map_err(Into::into)
                    }
                }
                WinError::ERROR_INVALID_NAME => {
                    // If source contains trailing slashes, check if we are dealing with
                    // a file instead of a dir, and if so, throw ENOTDIR.
                    if let Some(path) = strip_trailing_slashes_and_concatenate(&resolved_old)? {
                        if path.is_file() {
                            return Err(Error::ENOTDIR);
                        }
                    }
                    Err(WinError::ERROR_INVALID_NAME.into())
                }
                e => Err(e.into()),
            }
        }
        None => {
            log::debug!("Inconvertible OS error: {}", e);
            Err(Error::EIO)
        }
    })
}

pub(crate) fn num_hardlinks(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.nNumberOfLinks.into())
}

pub(crate) fn device_id(file: &File, _metadata: &Metadata) -> io::Result<u64> {
    Ok(winx::file::get_fileinfo(file)?.dwVolumeSerialNumber.into())
}

pub(crate) fn file_serial_no(file: &File) -> io::Result<u64> {
    let info = winx::file::get_fileinfo(file)?;
    let high = info.nFileIndexHigh;
    let low = info.nFileIndexLow;
    let no = ((high as u64) << 32) | (low as u64);
    Ok(no)
}

pub(crate) fn change_time(file: &File, _metadata: &Metadata) -> io::Result<i64> {
    winx::file::change_time(file)
}

pub(crate) fn fd_filestat_get_impl(file: &std::fs::File) -> Result<wasi::__wasi_filestat_t> {
    let metadata = file.metadata()?;
    Ok(wasi::__wasi_filestat_t {
        dev: device_id(file, &metadata)?,
        ino: file_serial_no(file)?,
        nlink: num_hardlinks(file, &metadata)?.try_into()?, // u64 doesn't fit into u32
        size: metadata.len(),
        atim: systemtime_to_timestamp(metadata.accessed()?)?,
        ctim: change_time(file, &metadata)?.try_into()?, // i64 doesn't fit into u64
        mtim: systemtime_to_timestamp(metadata.modified()?)?,
        filetype: filetype_from_std(&metadata.file_type()).to_wasi(),
    })
}

pub(crate) fn filetype_from_std(ftype: &std::fs::FileType) -> FileType {
    if ftype.is_file() {
        FileType::RegularFile
    } else if ftype.is_dir() {
        FileType::Directory
    } else if ftype.is_symlink() {
        FileType::Symlink
    } else {
        FileType::Unknown
    }
}

pub(crate) fn path_filestat_get(
    resolved: PathGet,
    dirflags: wasi::__wasi_lookupflags_t,
) -> Result<wasi::__wasi_filestat_t> {
    let path = resolved.concatenate()?;
    let file = File::open(path)?;
    fd_filestat_get_impl(&file)
}

pub(crate) fn path_filestat_set_times(
    resolved: PathGet,
    dirflags: wasi::__wasi_lookupflags_t,
    st_atim: wasi::__wasi_timestamp_t,
    mut st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> Result<()> {
    use winx::file::AccessMode;
    let path = resolved.concatenate()?;
    let file = OpenOptions::new()
        .access_mode(AccessMode::FILE_WRITE_ATTRIBUTES.bits())
        .open(path)?;
    fd_filestat_set_times_impl(&file, st_atim, st_mtim, fst_flags)
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use std::os::windows::fs::{symlink_dir, symlink_file};
    use winx::winerror::WinError;

    let old_path = concatenate(resolved.dirfd(), Path::new(old_path))?;
    let new_path = resolved.concatenate()?;

    // try creating a file symlink
    symlink_file(&old_path, &new_path).or_else(|e| {
        match e.raw_os_error() {
            Some(e) => {
                log::debug!("path_symlink at symlink_file error code={:?}", e);
                match WinError::from_u32(e as u32) {
                    WinError::ERROR_NOT_A_REPARSE_POINT => {
                        // try creating a dir symlink instead
                        symlink_dir(old_path, new_path).map_err(Into::into)
                    }
                    WinError::ERROR_ACCESS_DENIED => {
                        // does the target exist?
                        if new_path.exists() {
                            Err(Error::EEXIST)
                        } else {
                            Err(WinError::ERROR_ACCESS_DENIED.into())
                        }
                    }
                    WinError::ERROR_INVALID_NAME => {
                        // does the target without trailing slashes exist?
                        if let Some(path) = strip_trailing_slashes_and_concatenate(&resolved)? {
                            if path.exists() {
                                return Err(Error::EEXIST);
                            }
                        }
                        Err(WinError::ERROR_INVALID_NAME.into())
                    }
                    e => Err(e.into()),
                }
            }
            None => {
                log::debug!("Inconvertible OS error: {}", e);
                Err(Error::EIO)
            }
        }
    })
}

pub(crate) fn path_unlink_file(resolved: PathGet) -> Result<()> {
    use std::fs;
    use winx::winerror::WinError;

    let path = resolved.concatenate()?;
    let file_type = path
        .symlink_metadata()
        .map(|metadata| metadata.file_type())?;

    // check if we're unlinking a symlink
    // NB this will get cleaned up a lot when [std::os::windows::fs::FileTypeExt]
    // stabilises
    //
    // [std::os::windows::fs::FileTypeExt]: https://doc.rust-lang.org/std/os/windows/fs/trait.FileTypeExt.html
    if file_type.is_symlink() {
        fs::remove_file(&path).or_else(|e| {
            match e.raw_os_error() {
                Some(e) => {
                    log::debug!("path_unlink_file at symlink_file error code={:?}", e);
                    match WinError::from_u32(e as u32) {
                        WinError::ERROR_ACCESS_DENIED => {
                            // try unlinking a dir symlink instead
                            fs::remove_dir(path).map_err(Into::into)
                        }
                        e => Err(e.into()),
                    }
                }
                None => {
                    log::debug!("Inconvertible OS error: {}", e);
                    Err(Error::EIO)
                }
            }
        })
    } else if file_type.is_dir() {
        Err(Error::EISDIR)
    } else if file_type.is_file() {
        fs::remove_file(path).map_err(Into::into)
    } else {
        Err(Error::EINVAL)
    }
}

pub(crate) fn path_remove_directory(resolved: PathGet) -> Result<()> {
    let path = resolved.concatenate()?;
    std::fs::remove_dir(&path).map_err(Into::into)
}
