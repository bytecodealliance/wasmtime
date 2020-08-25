use crate::handle::{Handle, HandleRights};
use crate::sys::osdir::OsDir;
use crate::sys::{fd, AsFile};
use crate::wasi::{types, Errno, Result};
use std::convert::TryFrom;
use std::ffi::{OsStr, OsString};
use std::fs::{self, Metadata, OpenOptions};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use winapi::shared::winerror;
use winx::file::AccessMode;

fn strip_trailing_slashes_and_concatenate(dirfd: &OsDir, path: &str) -> Result<Option<PathBuf>> {
    if path.ends_with('/') {
        let suffix = path.trim_end_matches('/');
        concatenate(dirfd, Path::new(suffix)).map(Some)
    } else {
        Ok(None)
    }
}

fn strip_extended_prefix<P: AsRef<OsStr>>(path: P) -> OsString {
    let path: Vec<u16> = path.as_ref().encode_wide().collect();
    if &[92, 92, 63, 92] == &path[0..4] {
        OsString::from_wide(&path[4..])
    } else {
        OsString::from_wide(&path)
    }
}

fn concatenate<P: AsRef<Path>>(file: &OsDir, path: P) -> Result<PathBuf> {
    use winx::file::get_file_path;

    // WASI is not able to deal with absolute paths
    // so error out if absolute
    if path.as_ref().is_absolute() {
        return Err(Errno::Notcapable);
    }

    let dir_path = get_file_path(&*file.as_file()?)?;
    // concatenate paths
    let mut out_path = PathBuf::from(dir_path);
    out_path.push(path.as_ref());
    // strip extended prefix; otherwise we will error out on any relative
    // components with `out_path`
    let out_path = PathBuf::from(strip_extended_prefix(out_path));

    tracing::debug!("out_path={:?}", out_path);

    Ok(out_path)
}

fn file_access_mode_from_fdflags(fdflags: types::Fdflags, read: bool, write: bool) -> AccessMode {
    let mut access_mode = AccessMode::READ_CONTROL;

    // We always need `FILE_WRITE_ATTRIBUTES` so that we can set attributes such as filetimes, etc.
    access_mode.insert(AccessMode::FILE_WRITE_ATTRIBUTES);

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

/// Creates owned WASI path from OS string.
///
/// NB WASI spec requires OS string to be valid UTF-8. Otherwise,
/// `__WASI_ERRNO_ILSEQ` error is returned.
pub(crate) fn from_host<S: AsRef<OsStr>>(s: S) -> Result<String> {
    let vec: Vec<u16> = s.as_ref().encode_wide().collect();
    let s = String::from_utf16(&vec)?;
    Ok(s)
}

pub(crate) fn open_rights(
    input_rights: &HandleRights,
    oflags: types::Oflags,
    fdflags: types::Fdflags,
) -> HandleRights {
    // which rights are needed on the dirfd?
    let mut needed_base = types::Rights::PATH_OPEN;
    let mut needed_inheriting = input_rights.base | input_rights.inheriting;

    // convert open flags
    if oflags.contains(&types::Oflags::CREAT) {
        needed_base |= types::Rights::PATH_CREATE_FILE;
    } else if oflags.contains(&types::Oflags::TRUNC) {
        needed_base |= types::Rights::PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    if fdflags.contains(&types::Fdflags::DSYNC)
        || fdflags.contains(&types::Fdflags::RSYNC)
        || fdflags.contains(&types::Fdflags::SYNC)
    {
        needed_inheriting |= types::Rights::FD_DATASYNC;
        needed_inheriting |= types::Rights::FD_SYNC;
    }

    HandleRights::new(needed_base, needed_inheriting)
}

pub(crate) fn readlinkat(dirfd: &OsDir, s_path: &str) -> Result<String> {
    use winx::file::get_file_path;

    let path = concatenate(dirfd, Path::new(s_path))?;
    let err = match path.read_link() {
        Ok(target_path) => {
            // since on Windows we are effectively emulating 'at' syscalls
            // we need to strip the prefix from the absolute path
            // as otherwise we will error out since WASI is not capable
            // of dealing with absolute paths
            let dir_path = get_file_path(&*dirfd.as_file()?)?;
            let dir_path = PathBuf::from(strip_extended_prefix(dir_path));
            let target_path = target_path
                .strip_prefix(dir_path)
                .map_err(|_| Errno::Notcapable)?;
            let target_path = target_path.to_str().ok_or(Errno::Ilseq)?;
            return Ok(target_path.to_owned());
        }
        Err(e) => e,
    };
    if let Some(code) = err.raw_os_error() {
        tracing::debug!("readlinkat error={:?}", code);
        if code as u32 == winerror::ERROR_INVALID_NAME {
            if s_path.ends_with('/') {
                // strip "/" and check if exists
                let path = concatenate(dirfd, Path::new(s_path.trim_end_matches('/')))?;
                if path.exists() && !path.is_dir() {
                    return Err(Errno::Notdir);
                }
            }
        }
    }
    Err(err.into())
}

pub(crate) fn create_directory(file: &OsDir, path: &str) -> Result<()> {
    let path = concatenate(file, path)?;
    std::fs::create_dir(&path)?;
    Ok(())
}

pub(crate) fn link(
    old_dirfd: &OsDir,
    old_path: &str,
    new_dirfd: &OsDir,
    new_path: &str,
    follow_symlinks: bool,
) -> Result<()> {
    use std::fs;
    let mut old_path = concatenate(old_dirfd, old_path)?;
    let new_path = concatenate(new_dirfd, new_path)?;
    if follow_symlinks {
        // in particular, this will return an error if the target path doesn't exist
        tracing::debug!(
            old_path = tracing::field::display(&old_path),
            "Following symlinks"
        );
        old_path = fs::canonicalize(&old_path).map_err(|e| match e.raw_os_error() {
            // fs::canonicalize under Windows will return:
            // * ERROR_FILE_NOT_FOUND, if it encounters a dangling symlink
            // * ERROR_CANT_RESOLVE_FILENAME, if it encounters a symlink loop
            Some(code) if code as u32 == winerror::ERROR_CANT_RESOLVE_FILENAME => Errno::Loop,
            _ => e.into(),
        })?;
    }
    let err = match fs::hard_link(&old_path, &new_path) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    if let Some(code) = err.raw_os_error() {
        tracing::debug!("path_link at fs::hard_link error code={:?}", code);
        if code as u32 == winerror::ERROR_ACCESS_DENIED {
            // If an attempt is made to create a hard link to a directory, POSIX-compliant
            // implementations of link return `EPERM`, but `ERROR_ACCESS_DENIED` is converted
            // to `EACCES`. We detect and correct this case here.
            if fs::metadata(&old_path).map(|m| m.is_dir()).unwrap_or(false) {
                return Err(Errno::Perm);
            }
        }
    }
    Err(err.into())
}

pub(crate) fn open(
    dirfd: &OsDir,
    path: &str,
    read: bool,
    write: bool,
    oflags: types::Oflags,
    fdflags: types::Fdflags,
) -> Result<Box<dyn Handle>> {
    use winx::file::{AccessMode, CreationDisposition, Flags};

    let is_trunc = oflags.contains(&types::Oflags::TRUNC);

    if is_trunc {
        // Windows does not support append mode when opening for truncation
        // This is because truncation requires `GENERIC_WRITE` access, which will override the removal
        // of the `FILE_WRITE_DATA` permission.
        if fdflags.contains(&types::Fdflags::APPEND) {
            return Err(Errno::Notsup);
        }
    }

    // convert open flags
    // note: the calls to `write(true)` are to bypass an internal OpenOption check
    // the write flag will ultimately be ignored when `access_mode` is calculated below.
    let mut opts = OpenOptions::new();
    match oflags.into() {
        CreationDisposition::CREATE_ALWAYS => {
            opts.create(true).truncate(true).write(true);
        }
        CreationDisposition::CREATE_NEW => {
            opts.create_new(true).write(true);
        }
        CreationDisposition::TRUNCATE_EXISTING => {
            opts.truncate(true).write(true);
        }
        _ => {}
    }
    let path = concatenate(dirfd, path)?;
    match path.symlink_metadata().map(|metadata| metadata.file_type()) {
        Ok(file_type) => {
            // check if we are trying to open a symlink
            if file_type.is_symlink() {
                return Err(Errno::Loop);
            }
            // check if we are trying to open a file as a dir
            if file_type.is_file() && oflags.contains(&types::Oflags::DIRECTORY) {
                return Err(Errno::Notdir);
            }
        }
        Err(err) => match err.raw_os_error() {
            Some(code) => {
                tracing::debug!("path_open at symlink_metadata error code={:?}", code);
                match code as u32 {
                    winerror::ERROR_FILE_NOT_FOUND => {
                        // file not found, let it proceed to actually
                        // trying to open it
                    }
                    winerror::ERROR_INVALID_NAME => {
                        // TODO rethink this. For now, migrate how we handled
                        // it in `path::openat` on Windows.
                        return Err(Errno::Notdir);
                    }
                    _ => return Err(err.into()),
                };
            }
            None => {
                tracing::debug!("Inconvertible OS error: {}", err);
                return Err(Errno::Io);
            }
        },
    }

    let mut access_mode = file_access_mode_from_fdflags(fdflags, read, write);

    // Truncation requires the special `GENERIC_WRITE` bit set (this is why it doesn't work with append-only mode)
    if is_trunc {
        access_mode |= AccessMode::GENERIC_WRITE;
    }

    let flags: Flags = fdflags.into();
    let file = opts
        .access_mode(access_mode.bits())
        .custom_flags(flags.bits())
        .open(&path)?;
    let handle = <Box<dyn Handle>>::try_from(file)?;
    Ok(handle)
}

pub(crate) fn readlink(dirfd: &OsDir, path: &str, buf: &mut [u8]) -> Result<usize> {
    use winx::file::get_file_path;

    let path = concatenate(dirfd, path)?;
    let target_path = path.read_link()?;

    // since on Windows we are effectively emulating 'at' syscalls
    // we need to strip the prefix from the absolute path
    // as otherwise we will error out since WASI is not capable
    // of dealing with absolute paths
    let dir_path = get_file_path(&*dirfd.as_file()?)?;
    let dir_path = PathBuf::from(strip_extended_prefix(dir_path));
    let target_path = target_path
        .strip_prefix(dir_path)
        .map_err(|_| Errno::Notcapable)
        .and_then(|path| path.to_str().map(String::from).ok_or(Errno::Ilseq))?;

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

pub(crate) fn rename(
    old_dirfd: &OsDir,
    old_path_: &str,
    new_dirfd: &OsDir,
    new_path_: &str,
) -> Result<()> {
    use std::fs;

    let old_path = concatenate(old_dirfd, old_path_)?;
    let new_path = concatenate(new_dirfd, new_path_)?;

    // First sanity check: check we're not trying to rename dir to file or vice versa.
    // NB on Windows, the former is actually permitted [std::fs::rename].
    //
    // [std::fs::rename]: https://doc.rust-lang.org/std/fs/fn.rename.html
    if old_path.is_dir() && new_path.is_file() {
        return Err(Errno::Notdir);
    }
    // Second sanity check: check we're not trying to rename a file into a path
    // ending in a trailing slash.
    if old_path.is_file() && new_path_.ends_with('/') {
        return Err(Errno::Notdir);
    }

    // TODO handle symlinks
    let err = match fs::rename(&old_path, &new_path) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    match err.raw_os_error() {
        Some(code) => {
            tracing::debug!("path_rename at rename error code={:?}", code);
            match code as u32 {
                winerror::ERROR_ACCESS_DENIED => {
                    // So most likely dealing with new_path == dir.
                    // Eliminate case old_path == file first.
                    if old_path.is_file() {
                        return Err(Errno::Isdir);
                    } else {
                        // Ok, let's try removing an empty dir at new_path if it exists
                        // and is a nonempty dir.
                        fs::remove_dir(&new_path)?;
                        fs::rename(old_path, new_path)?;
                        return Ok(());
                    }
                }
                winerror::ERROR_INVALID_NAME => {
                    // If source contains trailing slashes, check if we are dealing with
                    // a file instead of a dir, and if so, throw ENOTDIR.
                    if let Some(path) =
                        strip_trailing_slashes_and_concatenate(old_dirfd, old_path_)?
                    {
                        if path.is_file() {
                            return Err(Errno::Notdir);
                        }
                    }
                }
                _ => {}
            }

            Err(err.into())
        }
        None => {
            tracing::debug!("Inconvertible OS error: {}", err);
            Err(Errno::Io)
        }
    }
}

pub(crate) fn symlink(old_path: &str, new_dirfd: &OsDir, new_path_: &str) -> Result<()> {
    use std::os::windows::fs::{symlink_dir, symlink_file};

    let old_path = concatenate(new_dirfd, Path::new(old_path))?;
    let new_path = concatenate(new_dirfd, new_path_)?;

    // Windows distinguishes between file and directory symlinks.
    // If the source doesn't exist or is an exotic file type, we fall back
    // to regular file symlinks.
    let use_dir_symlink = fs::metadata(&new_path)
        .as_ref()
        .map(Metadata::is_dir)
        .unwrap_or(false);

    let res = if use_dir_symlink {
        symlink_dir(&old_path, &new_path)
    } else {
        symlink_file(&old_path, &new_path)
    };

    let err = match res {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    match err.raw_os_error() {
        Some(code) => {
            tracing::debug!("path_symlink at symlink_file error code={:?}", code);
            match code as u32 {
                // If the target contains a trailing slash, the Windows API returns
                // ERROR_INVALID_NAME (which corresponds to ENOENT) instead of
                // ERROR_ALREADY_EXISTS (which corresponds to EEXIST)
                //
                // This concerns only trailing slashes (not backslashes) and
                // only symbolic links (not hard links).
                //
                // Since POSIX will return EEXIST in such case, we simulate this behavior
                winerror::ERROR_INVALID_NAME => {
                    if let Some(path) =
                        strip_trailing_slashes_and_concatenate(new_dirfd, new_path_)?
                    {
                        if path.exists() {
                            return Err(Errno::Exist);
                        }
                    }
                }
                _ => {}
            }

            Err(err.into())
        }
        None => {
            tracing::debug!("Inconvertible OS error: {}", err);
            Err(Errno::Io)
        }
    }
}

pub(crate) fn unlink_file(dirfd: &OsDir, path: &str) -> Result<()> {
    use std::fs;

    let path = concatenate(dirfd, path)?;
    let file_type = path
        .symlink_metadata()
        .map(|metadata| metadata.file_type())?;

    // check if we're unlinking a symlink
    // NB this will get cleaned up a lot when [std::os::windows::fs::FileTypeExt]
    // stabilises
    //
    // [std::os::windows::fs::FileTypeExt]: https://doc.rust-lang.org/std/os/windows/fs/trait.FileTypeExt.html
    if file_type.is_symlink() {
        let err = match fs::remove_file(&path) {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };
        match err.raw_os_error() {
            Some(code) => {
                tracing::debug!("path_unlink_file at symlink_file error code={:?}", code);
                if code as u32 == winerror::ERROR_ACCESS_DENIED {
                    // try unlinking a dir symlink instead
                    return fs::remove_dir(path).map_err(Into::into);
                }

                Err(err.into())
            }
            None => {
                tracing::debug!("Inconvertible OS error: {}", err);
                Err(Errno::Io)
            }
        }
    } else if file_type.is_dir() {
        Err(Errno::Isdir)
    } else if file_type.is_file() {
        fs::remove_file(path).map_err(Into::into)
    } else {
        Err(Errno::Inval)
    }
}

pub(crate) fn remove_directory(dirfd: &OsDir, path: &str) -> Result<()> {
    let path = concatenate(dirfd, path)?;
    std::fs::remove_dir(&path).map_err(Into::into)
}

pub(crate) fn filestat_get_at(dirfd: &OsDir, path: &str, follow: bool) -> Result<types::Filestat> {
    use winx::file::Flags;
    let path = concatenate(dirfd, path)?;
    let mut opts = OpenOptions::new();

    if !follow {
        // By specifying FILE_FLAG_OPEN_REPARSE_POINT, we force Windows to *not* dereference symlinks.
        opts.custom_flags(Flags::FILE_FLAG_OPEN_REPARSE_POINT.bits());
    }

    let file = opts.read(true).open(path)?;
    let stat = fd::filestat_get(&file)?;
    Ok(stat)
}

pub(crate) fn filestat_set_times_at(
    dirfd: &OsDir,
    path: &str,
    atim: types::Timestamp,
    mtim: types::Timestamp,
    fst_flags: types::Fstflags,
    follow: bool,
) -> Result<()> {
    use winx::file::{AccessMode, Flags};
    let path = concatenate(dirfd, path)?;
    let mut opts = OpenOptions::new();

    if !follow {
        // By specifying FILE_FLAG_OPEN_REPARSE_POINT, we force Windows to *not* dereference symlinks.
        opts.custom_flags(Flags::FILE_FLAG_OPEN_REPARSE_POINT.bits());
    }

    let file = opts
        .access_mode(AccessMode::FILE_WRITE_ATTRIBUTES.bits())
        .open(path)?;
    fd::filestat_set_times(&file, atim, mtim, fst_flags)?;
    Ok(())
}
