pub(crate) mod filetime;
pub(crate) mod hostcalls_impl;
pub(crate) mod oshandle;

pub(crate) mod host_impl {
    use crate::{wasi, Result};
    use std::convert::TryFrom;

    pub(crate) const O_RSYNC: yanix::file::OFlag = yanix::file::OFlag::SYNC;

    pub(crate) fn stdev_from_nix(dev: libc::dev_t) -> Result<wasi::__wasi_device_t> {
        wasi::__wasi_device_t::try_from(dev).map_err(Into::into)
    }

    pub(crate) fn stino_from_nix(ino: libc::ino_t) -> Result<wasi::__wasi_inode_t> {
        wasi::__wasi_device_t::try_from(ino).map_err(Into::into)
    }
}
