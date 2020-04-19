use crate::sys::osdir::OsDir;
use crate::wasi::Result;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn unlink_file(dirfd: &OsDir, path: &str) -> Result<()> {
    use yanix::file::{unlinkat, AtFlag};
    unsafe { unlinkat(dirfd.as_raw_fd(), path, AtFlag::empty())? };
    Ok(())
}

pub(crate) fn symlink(old_path: &str, new_dirfd: &OsDir, new_path: &str) -> Result<()> {
    use yanix::file::symlinkat;

    log::debug!("path_symlink old_path = {:?}", old_path);
    log::debug!(
        "path_symlink (new_dirfd, new_path) = ({:?}, {:?})",
        new_dirfd,
        new_path
    );

    unsafe { symlinkat(old_path, new_dirfd.as_raw_fd(), new_path)? };
    Ok(())
}

pub(crate) fn rename(
    old_dirfd: &OsDir,
    old_path: &str,
    new_dirfd: &OsDir,
    new_path: &str,
) -> Result<()> {
    use yanix::file::renameat;
    unsafe {
        renameat(
            old_dirfd.as_raw_fd(),
            old_path,
            new_dirfd.as_raw_fd(),
            new_path,
        )?
    };
    Ok(())
}
