use crate::old::snapshot_0::wasi::{self, WasiResult};

pub(crate) const O_RSYNC: yanix::file::OFlags = yanix::file::OFlags::RSYNC;

pub(crate) fn stdev_from_nix(dev: libc::dev_t) -> WasiResult<wasi::__wasi_device_t> {
    Ok(wasi::__wasi_device_t::from(dev))
}

pub(crate) fn stino_from_nix(ino: libc::ino_t) -> WasiResult<wasi::__wasi_inode_t> {
    Ok(wasi::__wasi_device_t::from(ino))
}

pub(crate) fn stnlink_from_nix(nlink: libc::nlink_t) -> WasiResult<wasi::__wasi_linkcount_t> {
    Ok(nlink)
}
