use crate::old::snapshot_0::hostcalls_impl::PathGet;
use crate::old::snapshot_0::wasi::{WasiError, WasiResult};
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_unlink_file(resolved: PathGet) -> WasiResult<()> {
    use yanix::file::{unlinkat, AtFlag};
    match unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            resolved.path(),
            AtFlag::empty(),
        )
    } {
        Err(err) => {
            let raw_errno = err.raw_os_error().unwrap();
            // Non-Linux implementations may return EPERM when attempting to remove a
            // directory without REMOVEDIR. While that's what POSIX specifies, it's
            // less useful. Adjust this to EISDIR. It doesn't matter that this is not
            // atomic with the unlinkat, because if the file is removed and a directory
            // is created before fstatat sees it, we're racing with that change anyway
            // and unlinkat could have legitimately seen the directory if the race had
            // turned out differently.
            use yanix::file::{fstatat, FileType};

            if raw_errno == libc::EPERM {
                match unsafe {
                    fstatat(
                        resolved.dirfd().as_raw_fd(),
                        resolved.path(),
                        AtFlag::SYMLINK_NOFOLLOW,
                    )
                } {
                    Ok(stat) => {
                        if FileType::from_stat_st_mode(stat.st_mode) == FileType::Directory {
                            return Err(WasiError::EISDIR);
                        }
                    }
                    Err(err) => {
                        log::debug!("path_unlink_file fstatat error: {:?}", err);
                    }
                }
            }

            Err(err.into())
        }
        Ok(()) => Ok(()),
    }
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> WasiResult<()> {
    use yanix::file::{fstatat, symlinkat, AtFlag};

    log::debug!("path_symlink old_path = {:?}", old_path);
    log::debug!("path_symlink resolved = {:?}", resolved);

    match unsafe { symlinkat(old_path, resolved.dirfd().as_raw_fd(), resolved.path()) } {
        Err(err) => {
            if err.raw_os_error().unwrap() == libc::ENOTDIR {
                // On BSD, symlinkat returns ENOTDIR when it should in fact
                // return a EEXIST. It seems that it gets confused with by
                // the trailing slash in the target path. Thus, we strip
                // the trailing slash and check if the path exists, and
                // adjust the error code appropriately.
                let new_path = resolved.path().trim_end_matches('/');
                match unsafe {
                    fstatat(
                        resolved.dirfd().as_raw_fd(),
                        new_path,
                        AtFlag::SYMLINK_NOFOLLOW,
                    )
                } {
                    Ok(_) => return Err(WasiError::EEXIST),
                    Err(err) => {
                        log::debug!("path_symlink fstatat error: {:?}", err);
                    }
                }
            }
            Err(err.into())
        }
        Ok(()) => Ok(()),
    }
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> WasiResult<()> {
    use yanix::file::{fstatat, renameat, AtFlag};
    match unsafe {
        renameat(
            resolved_old.dirfd().as_raw_fd(),
            resolved_old.path(),
            resolved_new.dirfd().as_raw_fd(),
            resolved_new.path(),
        )
    } {
        Err(err) => {
            // Currently, this is verified to be correct on macOS, where
            // ENOENT can be returned in case when we try to rename a file
            // into a name with a trailing slash. On macOS, if the latter does
            // not exist, an ENOENT is thrown, whereas on Linux we observe the
            // correct behaviour of throwing an ENOTDIR since the destination is
            // indeed not a directory.
            //
            // TODO
            // Verify on other BSD-based OSes.
            if err.raw_os_error().unwrap() == libc::ENOENT {
                // check if the source path exists
                match unsafe {
                    fstatat(
                        resolved_old.dirfd().as_raw_fd(),
                        resolved_old.path(),
                        AtFlag::SYMLINK_NOFOLLOW,
                    )
                } {
                    Ok(_) => {
                        // check if destination contains a trailing slash
                        if resolved_new.path().contains('/') {
                            return Err(WasiError::ENOTDIR);
                        } else {
                            return Err(WasiError::ENOENT);
                        }
                    }
                    Err(err) => {
                        log::debug!("path_rename fstatat error: {:?}", err);
                    }
                }
            }

            Err(err.into())
        }
        Ok(()) => Ok(()),
    }
}

pub(crate) mod fd_readdir_impl {
    use crate::old::snapshot_0::sys::fdentry_impl::OsHandle;
    use crate::old::snapshot_0::wasi::WasiResult;
    use std::sync::{Mutex, MutexGuard};
    use yanix::dir::Dir;

    pub(crate) fn get_dir_from_os_handle<'a>(
        os_handle: &'a mut OsHandle,
    ) -> WasiResult<MutexGuard<'a, Dir>> {
        let dir = match os_handle.dir {
            Some(ref mut dir) => dir,
            None => {
                // We need to duplicate the fd, because `opendir(3)`:
                //     Upon successful return from fdopendir(), the file descriptor is under
                //     control of the system, and if any attempt is made to close the file
                //     descriptor, or to modify the state of the associated description other
                //     than by means of closedir(), readdir(), readdir_r(), or rewinddir(),
                //     the behaviour is undefined.
                let fd = (*os_handle).try_clone()?;
                let dir = Dir::from(fd)?;
                os_handle.dir.get_or_insert(Mutex::new(dir))
            }
        };
        // Note that from this point on, until the end of the parent scope (i.e., enclosing this
        // function), we're locking the `Dir` member of this `OsHandle`.
        Ok(dir.lock().unwrap())
    }
}
