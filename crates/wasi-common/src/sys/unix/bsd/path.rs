use crate::sys::osdir::OsDir;
use crate::{Error, Result};
use std::os::unix::prelude::AsRawFd;

pub(crate) fn unlink_file(dirfd: &OsDir, path: &str) -> Result<()> {
    use yanix::file::{unlinkat, AtFlags};
    match unsafe { unlinkat(dirfd.as_raw_fd(), path, AtFlags::empty()) } {
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
                match unsafe { fstatat(dirfd.as_raw_fd(), path, AtFlags::SYMLINK_NOFOLLOW) } {
                    Ok(stat) => {
                        if FileType::from_stat_st_mode(stat.st_mode) == FileType::Directory {
                            return Err(Error::Isdir);
                        }
                    }
                    Err(err) => {
                        tracing::debug!("path_unlink_file fstatat error: {:?}", err);
                    }
                }
            }

            Err(err.into())
        }
        Ok(()) => Ok(()),
    }
}

pub(crate) fn symlink(old_path: &str, new_dirfd: &OsDir, new_path: &str) -> Result<()> {
    use yanix::file::{fstatat, symlinkat, AtFlags};

    tracing::debug!("path_symlink old_path = {:?}", old_path);
    tracing::debug!(
        "path_symlink (new_dirfd, new_path) = ({:?}, {:?})",
        new_dirfd,
        new_path
    );

    match unsafe { symlinkat(old_path, new_dirfd.as_raw_fd(), new_path) } {
        Err(err) => {
            if err.raw_os_error().unwrap() == libc::ENOTDIR {
                // On BSD, symlinkat returns ENOTDIR when it should in fact
                // return a EEXIST. It seems that it gets confused with by
                // the trailing slash in the target path. Thus, we strip
                // the trailing slash and check if the path exists, and
                // adjust the error code appropriately.
                let new_path = new_path.trim_end_matches('/');
                match unsafe { fstatat(new_dirfd.as_raw_fd(), new_path, AtFlags::SYMLINK_NOFOLLOW) }
                {
                    Ok(_) => return Err(Error::Exist),
                    Err(err) => {
                        tracing::debug!("path_symlink fstatat error: {:?}", err);
                    }
                }
            }
            Err(err.into())
        }
        Ok(()) => Ok(()),
    }
}

pub(crate) fn rename(
    old_dirfd: &OsDir,
    old_path: &str,
    new_dirfd: &OsDir,
    new_path: &str,
) -> Result<()> {
    use yanix::file::{fstatat, renameat, AtFlags};
    match unsafe {
        renameat(
            old_dirfd.as_raw_fd(),
            old_path,
            new_dirfd.as_raw_fd(),
            new_path,
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
                match unsafe { fstatat(old_dirfd.as_raw_fd(), old_path, AtFlags::SYMLINK_NOFOLLOW) }
                {
                    Ok(_) => {
                        // check if destination contains a trailing slash
                        if new_path.contains('/') {
                            return Err(Error::Notdir);
                        } else {
                            return Err(Error::Noent);
                        }
                    }
                    Err(err) => {
                        tracing::debug!("path_rename fstatat error: {:?}", err);
                    }
                }
            }

            Err(err.into())
        }
        Ok(()) => Ok(()),
    }
}
