#![allow(non_camel_case_types)]
use crate::old::snapshot_0::hostcalls_impl::PathGet;
use crate::old::snapshot_0::wasi::{self, WasiError, WasiResult};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use winapi::shared::winerror;

pub(crate) trait PathGetExt {
    fn concatenate(&self) -> WasiResult<PathBuf>;
}

impl PathGetExt for PathGet {
    fn concatenate(&self) -> WasiResult<PathBuf> {
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

pub(crate) fn openat(dirfd: &File, path: &str) -> WasiResult<File> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use winx::file::Flags;

    let path = concatenate(dirfd, Path::new(path))?;
    let err = match OpenOptions::new()
        .read(true)
        .custom_flags(Flags::FILE_FLAG_BACKUP_SEMANTICS.bits())
        .open(&path)
    {
        Ok(file) => return Ok(file),
        Err(e) => e,
    };
    if let Some(code) = err.raw_os_error() {
        log::debug!("openat error={:?}", code);
        if code as u32 == winerror::ERROR_INVALID_NAME {
            return Err(WasiError::ENOTDIR);
        }
    }
    Err(err.into())
}

pub(crate) fn readlinkat(dirfd: &File, s_path: &str) -> WasiResult<String> {
    use winx::file::get_file_path;

    let path = concatenate(dirfd, Path::new(s_path))?;
    let err = match path.read_link() {
        Ok(target_path) => {
            // since on Windows we are effectively emulating 'at' syscalls
            // we need to strip the prefix from the absolute path
            // as otherwise we will error out since WASI is not capable
            // of dealing with absolute paths
            let dir_path = get_file_path(dirfd)?;
            let dir_path = PathBuf::from(strip_extended_prefix(dir_path));
            let target_path = target_path
                .strip_prefix(dir_path)
                .map_err(|_| WasiError::ENOTCAPABLE)?;
            let target_path = target_path.to_str().ok_or(WasiError::EILSEQ)?;
            return Ok(target_path.to_owned());
        }
        Err(e) => e,
    };
    if let Some(code) = err.raw_os_error() {
        log::debug!("readlinkat error={:?}", code);
        if code as u32 == winerror::ERROR_INVALID_NAME {
            if s_path.ends_with('/') {
                // strip "/" and check if exists
                let path = concatenate(dirfd, Path::new(s_path.trim_end_matches('/')))?;
                if path.exists() && !path.is_dir() {
                    return Err(WasiError::ENOTDIR);
                }
            }
        }
    }
    Err(err.into())
}

pub(crate) fn strip_extended_prefix<P: AsRef<OsStr>>(path: P) -> OsString {
    let path: Vec<u16> = path.as_ref().encode_wide().collect();
    if &[92, 92, 63, 92] == &path[0..4] {
        OsString::from_wide(&path[4..])
    } else {
        OsString::from_wide(&path)
    }
}

pub(crate) fn concatenate<P: AsRef<Path>>(dirfd: &File, path: P) -> WasiResult<PathBuf> {
    use winx::file::get_file_path;

    // WASI is not able to deal with absolute paths
    // so error out if absolute
    if path.as_ref().is_absolute() {
        return Err(WasiError::ENOTCAPABLE);
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
