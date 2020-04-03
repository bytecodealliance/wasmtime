use crate::entry::Descriptor;
use crate::path::PathGet;
use crate::wasi::Result;
use std::os::unix::prelude::AsRawFd;

pub(crate) fn unlink_file(resolved: PathGet) -> Result<()> {
    use yanix::file::{unlinkat, AtFlag};
    unsafe {
        unlinkat(
            resolved.dirfd().as_raw_fd(),
            resolved.path(),
            AtFlag::empty(),
        )?
    };
    Ok(())
}

pub(crate) fn symlink(old_path: &str, resolved: PathGet) -> Result<()> {
    use yanix::file::symlinkat;

    log::debug!("path_symlink old_path = {:?}", old_path);
    log::debug!("path_symlink resolved = {:?}", resolved);

    unsafe { symlinkat(old_path, resolved.dirfd().as_raw_fd(), resolved.path())? };
    Ok(())
}

pub(crate) fn rename(resolved_old: PathGet, resolved_new: PathGet) -> Result<()> {
    use yanix::file::renameat;
    match (resolved_old.dirfd(), resolved_new.dirfd()) {
        (Descriptor::OsHandle(resolved_old_file), Descriptor::OsHandle(resolved_new_file)) => {
            unsafe {
                renameat(
                    resolved_old_file.as_raw_fd(),
                    resolved_old.path(),
                    resolved_new_file.as_raw_fd(),
                    resolved_new.path(),
                )?
            };
            Ok(())
        }
        _ => {
            unimplemented!("path_link with one or more virtual files");
        }
    }
}
