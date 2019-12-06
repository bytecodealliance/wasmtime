#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::sys::host_impl;
use crate::{wasi, Result};
use std::fs::File;
use yanix::file::OFlag;

pub(crate) fn path_open_rights(
    rights_base: wasi::__wasi_rights_t,
    rights_inheriting: wasi::__wasi_rights_t,
    oflags: wasi::__wasi_oflags_t,
    fs_flags: wasi::__wasi_fdflags_t,
) -> (wasi::__wasi_rights_t, wasi::__wasi_rights_t) {
    // which rights are needed on the dirfd?
    let mut needed_base = wasi::__WASI_RIGHTS_PATH_OPEN;
    let mut needed_inheriting = rights_base | rights_inheriting;

    // convert open flags
    let oflags = host_impl::nix_from_oflags(oflags);
    if oflags.contains(OFlag::CREAT) {
        needed_base |= wasi::__WASI_RIGHTS_PATH_CREATE_FILE;
    }
    if oflags.contains(OFlag::TRUNC) {
        needed_base |= wasi::__WASI_RIGHTS_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let fdflags = host_impl::nix_from_fdflags(fs_flags);
    if fdflags.contains(OFlag::DSYNC) {
        needed_inheriting |= wasi::__WASI_RIGHTS_FD_DATASYNC;
    }
    if fdflags.intersects(host_impl::O_RSYNC | OFlag::SYNC) {
        needed_inheriting |= wasi::__WASI_RIGHTS_FD_SYNC;
    }

    (needed_base, needed_inheriting)
}

pub(crate) fn openat(dirfd: &File, path: &str) -> Result<File> {
    use std::os::unix::prelude::{AsRawFd, FromRawFd};
    use yanix::file::{openat, Mode};

    log::debug!("path_get openat path = {:?}", path);

    unsafe {
        openat(
            dirfd.as_raw_fd(),
            path,
            OFlag::RDONLY | OFlag::DIRECTORY | OFlag::NOFOLLOW,
            Mode::empty(),
        )
    }
    .map(|new_fd| unsafe { File::from_raw_fd(new_fd) })
    .map_err(Into::into)
}

pub(crate) fn readlinkat(dirfd: &File, path: &str) -> Result<String> {
    use std::os::unix::prelude::AsRawFd;
    use yanix::file::readlinkat;

    log::debug!("path_get readlinkat path = {:?}", path);

    unsafe { readlinkat(dirfd.as_raw_fd(), path) }
        .map_err(Into::into)
        .and_then(host_impl::path_from_host)
}
