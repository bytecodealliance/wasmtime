//! Utilities for working with `/proc`, where Linux's `procfs` is typically
//! mounted. `/proc` serves as an adjunct to Linux's main syscall surface area,
//! providing additional features with an awkward interface.
//!
//! This module does a considerable amount of work to determine whether `/proc`
//! is mounted, with actual `procfs`, and without any additional mount points
//! on top of the paths we open.

use crate::filesystem::primitives::OpenOptionsExt;
use crate::filesystem::primitives::{OpenOptions, open, set_times_follow_unchecked};
use io_lifetimes::AsFd;
use rustix::fs::OFlags;
use rustix::path::DecInt;
use rustix_linux_procfs::proc_self_fd;
use std::path::Path;
use std::time::SystemTime;
use std::{fs, io};

pub(crate) fn set_times_through_proc_self_fd(
    start: &fs::File,
    path: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    let opath = open(
        start,
        path,
        OpenOptions::new()
            .read(true)
            .custom_flags(OFlags::PATH.bits() as i32),
    )?;

    // Don't pass `AT_SYMLINK_NOFOLLOW`, because we do actually want to follow
    // the first symlink. We don't want to follow any subsequent symlinks, but
    // omitting `O_NOFOLLOW` above ensures that the destination of the link
    // isn't a symlink.
    set_times_follow_unchecked(
        proc_self_fd()?.as_fd(),
        DecInt::from_fd(&opath).as_ref(),
        atime,
        mtime,
    )
}
