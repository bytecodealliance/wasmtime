use crate::old::snapshot_0::wasi::{self, WasiResult};
use std::convert::TryFrom;

pub(crate) const O_RSYNC: yanix::file::OFlag = yanix::file::OFlag::SYNC;

pub(crate) fn stdev_from_nix(dev: libc::dev_t) -> WasiResult<wasi::__wasi_device_t> {
    wasi::__wasi_device_t::try_from(dev).map_err(Into::into)
}

pub(crate) fn stino_from_nix(ino: libc::ino_t) -> WasiResult<wasi::__wasi_inode_t> {
    wasi::__wasi_device_t::try_from(ino).map_err(Into::into)
}

pub(crate) fn stnlink_from_nix(nlink: libc::nlink_t) -> WasiResult<wasi::__wasi_linkcount_t> {
    Ok(nlink.into())
}
