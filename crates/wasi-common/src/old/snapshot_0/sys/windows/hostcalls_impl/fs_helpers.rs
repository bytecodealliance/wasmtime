#![allow(non_camel_case_types)]
use crate::old::snapshot_0::hostcalls_impl::PathGet;
use crate::old::snapshot_0::{wasi, Error, Result};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

pub(crate) trait PathGetExt {
    fn concatenate(&self) -> Result<PathBuf>;
}

impl PathGetExt for PathGet {
    fn concatenate(&self) -> Result<PathBuf> {
        concatenate(self.dirfd(), Path::new(self.path()))
    }
}

pub(crate) fn path_open_rights(
    rights_base: wasi::__wasi_rights_t,
    rights_inheriting: wasi::__wasi_rights_t,
    oflags: wasi::__wasi_oflags_t,
    fdflags: wasi::__wasi_fdflags_t,
) -> (wasi::__wasi_rights_t, wasi::__wasi_rights_t) {
    // which rights are needed on the dirfd?
    let mut needed_base = wasi::__WASI_RIGHTS_PATH_OPEN;
    let mut needed_inheriting = rights_base | rights_inheriting;

    // convert open flags
    if oflags & wasi::__WASI_OFLAGS_CREAT != 0 {
        needed_base |= wasi::__WASI_RIGHTS_PATH_CREATE_FILE;
    } else if oflags & wasi::__WASI_OFLAGS_TRUNC != 0 {
        needed_base |= wasi::__WASI_RIGHTS_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    if fdflags & wasi::__WASI_FDFLAGS_DSYNC != 0
        || fdflags & wasi::__WASI_FDFLAGS_RSYNC != 0
        || fdflags & wasi::__WASI_FDFLAGS_SYNC != 0
    {
        needed_inheriting |= wasi::__WASI_RIGHTS_FD_DATASYNC;
        needed_inheriting |= wasi::__WASI_RIGHTS_FD_SYNC;
    }

    (needed_base, needed_inheriting)
}

pub(crate) fn openat(dirfd: &File, path: &str) -> Result<File> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use winx::file::Flags;
    use winx::winerror::WinError;

    let path = concatenate(dirfd, Path::new(path))?;
    OpenOptions::new()
        .read(true)
        .custom_flags(Flags::FILE_FLAG_BACKUP_SEMANTICS.bits())
        .open(&path)
        .map_err(|e| match e.raw_os_error() {
            Some(e) => {
                log::debug!("openat error={:?}", e);
                match WinError::from_u32(e as u32) {
                    WinError::ERROR_INVALID_NAME => Error::ENOTDIR,
                    e => e.into(),
                }
            }
            None => {
                log::debug!("Inconvertible OS error: {}", e);
                Error::EIO
            }
        })
}

pub(crate) fn readlinkat(dirfd: &File, s_path: &str) -> Result<String> {
    use winx::file::get_file_path;
    use winx::winerror::WinError;

    let path = concatenate(dirfd, Path::new(s_path))?;
    match path.read_link() {
        Ok(target_path) => {
            // since on Windows we are effectively emulating 'at' syscalls
            // we need to strip the prefix from the absolute path
            // as otherwise we will error out since WASI is not capable
            // of dealing with absolute paths
            let dir_path = get_file_path(dirfd)?;
            let dir_path = PathBuf::from(strip_extended_prefix(dir_path));
            target_path
                .strip_prefix(dir_path)
                .map_err(|_| Error::ENOTCAPABLE)
                .and_then(|path| path.to_str().map(String::from).ok_or(Error::EILSEQ))
        }
        Err(e) => match e.raw_os_error() {
            Some(e) => {
                log::debug!("readlinkat error={:?}", e);
                match WinError::from_u32(e as u32) {
                    WinError::ERROR_INVALID_NAME => {
                        if s_path.ends_with('/') {
                            // strip "/" and check if exists
                            let path = concatenate(dirfd, Path::new(s_path.trim_end_matches('/')))?;
                            if path.exists() && !path.is_dir() {
                                Err(Error::ENOTDIR)
                            } else {
                                Err(Error::ENOENT)
                            }
                        } else {
                            Err(Error::ENOENT)
                        }
                    }
                    e => Err(e.into()),
                }
            }
            None => {
                log::debug!("Inconvertible OS error: {}", e);
                Err(Error::EIO)
            }
        },
    }
}

pub(crate) fn strip_extended_prefix<P: AsRef<OsStr>>(path: P) -> OsString {
    let path: Vec<u16> = path.as_ref().encode_wide().collect();
    if &[92, 92, 63, 92] == &path[0..4] {
        OsString::from_wide(&path[4..])
    } else {
        OsString::from_wide(&path)
    }
}

pub(crate) fn concatenate<P: AsRef<Path>>(dirfd: &File, path: P) -> Result<PathBuf> {
    use winx::file::get_file_path;

    // WASI is not able to deal with absolute paths
    // so error out if absolute
    if path.as_ref().is_absolute() {
        return Err(Error::ENOTCAPABLE);
    }

    let dir_path = get_file_path(dirfd)?;
    // concatenate paths
    let mut out_path = PathBuf::from(dir_path);
    out_path.push(path.as_ref());
    // strip extended prefix; otherwise we will error out on any relative
    // components with `out_path`
    let out_path = PathBuf::from(strip_extended_prefix(out_path));

    log::debug!("out_path={:?}", out_path);

    Ok(out_path)
}
