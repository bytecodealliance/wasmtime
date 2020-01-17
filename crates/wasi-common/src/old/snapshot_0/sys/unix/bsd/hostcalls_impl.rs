use crate::old::snapshot_0::hostcalls_impl::PathGet;
use crate::old::snapshot_0::{Error, Result};
use std::os::unix::prelude::AsRawFd;

pub(crate) fn path_unlink_file(resolved: PathGet) -> Result<()> {
    use yanix::{
        file::{unlinkat, AtFlag},
        Errno, YanixError,
    };
    unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            resolved.path(),
            AtFlag::empty(),
        )
    }
    .map_err(|err| {
        if let YanixError::Errno(mut errno) = err {
            // Non-Linux implementations may return EPERM when attempting to remove a
            // directory without REMOVEDIR. While that's what POSIX specifies, it's
            // less useful. Adjust this to EISDIR. It doesn't matter that this is not
            // atomic with the unlinkat, because if the file is removed and a directory
            // is created before fstatat sees it, we're racing with that change anyway
            // and unlinkat could have legitimately seen the directory if the race had
            // turned out differently.
            use yanix::file::{fstatat, SFlag};

            if errno == Errno::EPERM {
                if let Ok(stat) = unsafe {
                    fstatat(
                        resolved.dirfd().as_raw_fd(),
                        resolved.path(),
                        AtFlag::SYMLINK_NOFOLLOW,
                    )
                } {
                    if SFlag::from_bits_truncate(stat.st_mode).contains(SFlag::IFDIR) {
                        errno = Errno::EISDIR;
                    }
                } else {
                    errno = Errno::last();
                }
            }
            errno.into()
        } else {
            err
        }
    })
    .map_err(Into::into)
}

pub(crate) fn path_symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use yanix::{
        file::{fstatat, symlinkat, AtFlag},
        Errno, YanixError,
    };

    log::debug!("path_symlink old_path = {:?}", old_path);
    log::debug!("path_symlink resolved = {:?}", resolved);

    unsafe { symlinkat(old_path, resolved.dirfd().as_raw_fd(), resolved.path()) }.or_else(|err| {
        if let YanixError::Errno(errno) = err {
            match errno {
                Errno::ENOTDIR => {
                    // On BSD, symlinkat returns ENOTDIR when it should in fact
                    // return a EEXIST. It seems that it gets confused with by
                    // the trailing slash in the target path. Thus, we strip
                    // the trailing slash and check if the path exists, and
                    // adjust the error code appropriately.
                    let new_path = resolved.path().trim_end_matches('/');
                    if let Ok(_) = unsafe {
                        fstatat(
                            resolved.dirfd().as_raw_fd(),
                            new_path,
                            AtFlag::SYMLINK_NOFOLLOW,
                        )
                    } {
                        Err(Error::EEXIST)
                    } else {
                        Err(Error::ENOTDIR)
                    }
                }
                x => Err(x.into()),
            }
        } else {
            Err(err.into())
        }
    })
}

pub(crate) fn path_rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use yanix::{
        file::{fstatat, renameat, AtFlag},
        Errno, YanixError,
    };
    unsafe {
        renameat(
            resolved_old.dirfd().as_raw_fd(),
            resolved_old.path(),
            resolved_new.dirfd().as_raw_fd(),
            resolved_new.path(),
        )
    }
    .or_else(|err| {
        // Currently, this is verified to be correct on macOS, where
        // ENOENT can be returned in case when we try to rename a file
        // into a name with a trailing slash. On macOS, if the latter does
        // not exist, an ENOENT is thrown, whereas on Linux we observe the
        // correct behaviour of throwing an ENOTDIR since the destination is
        // indeed not a directory.
        //
        // TODO
        // Verify on other BSD-based OSes.
        if let YanixError::Errno(errno) = err {
            match errno {
                Errno::ENOENT => {
                    // check if the source path exists
                    if let Ok(_) = unsafe {
                        fstatat(
                            resolved_old.dirfd().as_raw_fd(),
                            resolved_old.path(),
                            AtFlag::SYMLINK_NOFOLLOW,
                        )
                    } {
                        // check if destination contains a trailing slash
                        if resolved_new.path().contains('/') {
                            Err(Error::ENOTDIR)
                        } else {
                            Err(Error::ENOENT)
                        }
                    } else {
                        Err(Error::ENOENT)
                    }
                }
                x => Err(x.into()),
            }
        } else {
            Err(err.into())
        }
    })
}

pub(crate) mod fd_readdir_impl {
    use crate::old::snapshot_0::sys::fdentry_impl::OsHandle;
    use crate::old::snapshot_0::Result;
    use std::sync::{Mutex, MutexGuard};
    use yanix::dir::Dir;

    pub(crate) fn get_dir_from_os_handle<'a>(
        os_handle: &'a mut OsHandle,
    ) -> Result<MutexGuard<'a, Dir>> {
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
