#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
use crate::old::snapshot_0::sys::host_impl;
use crate::old::snapshot_0::wasi::{self, WasiResult};
use std::fs::File;
use yanix::file::OFlags;

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
    if oflags.contains(OFlags::CREAT) {
        needed_base |= wasi::__WASI_RIGHTS_PATH_CREATE_FILE;
    }
    if oflags.contains(OFlags::TRUNC) {
        needed_base |= wasi::__WASI_RIGHTS_PATH_FILESTAT_SET_SIZE;
    }

    // convert file descriptor flags
    let fdflags = host_impl::nix_from_fdflags(fs_flags);
    if fdflags.contains(OFlags::DSYNC) {
        needed_inheriting |= wasi::__WASI_RIGHTS_FD_DATASYNC;
    }
    if fdflags.intersects(host_impl::O_RSYNC | OFlags::SYNC) {
        needed_inheriting |= wasi::__WASI_RIGHTS_FD_SYNC;
    }

    (needed_base, needed_inheriting)
}

pub(crate) fn openat(dirfd: &File, path: &str) -> WasiResult<File> {
    use std::os::unix::prelude::{AsRawFd, FromRawFd};
    use yanix::file::{openat, Mode};

    tracing::debug!("path_get openat path = {:?}", path);

    unsafe {
        openat(
            dirfd.as_raw_fd(),
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW,
            Mode::empty(),
        )
    }
    .map(|new_fd| unsafe { File::from_raw_fd(new_fd) })
    .map_err(Into::into)
}

pub(crate) fn readlinkat(dirfd: &File, path: &str) -> WasiResult<String> {
    use std::os::unix::prelude::AsRawFd;
    use yanix::file::readlinkat;

    tracing::debug!("path_get readlinkat path = {:?}", path);

    unsafe { readlinkat(dirfd.as_raw_fd(), path) }
        .map_err(Into::into)
        .and_then(host_impl::path_from_host)
}
