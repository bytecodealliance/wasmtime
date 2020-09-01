use crate::sys::osdir::OsDir;
use crate::Result;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn unlink_file(dirfd: &OsDir, path: &str) -> Result<()> {
    use yanix::file::{unlinkat, AtFlags};
    unsafe { unlinkat(dirfd.as_raw_fd(), path, AtFlags::empty())? };
    Ok(())
}

pub(crate) fn symlink(old_path: &str, new_dirfd: &OsDir, new_path: &str) -> Result<()> {
    use yanix::file::symlinkat;

    tracing::debug!(
        old_path = old_path,
        new_dirfd = tracing::field::debug(new_dirfd),
        new_path = new_path,
        "path symlink"
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
