pub(crate) mod hostcalls_impl;
pub(crate) mod oshandle;

pub(crate) mod host_impl {
    use crate::{wasi, Result};

    pub(crate) const O_RSYNC: yanix::file::OFlag = yanix::file::OFlag::RSYNC;

    pub(crate) fn stdev_from_nix(dev: libc::dev_t) -> Result<wasi::__wasi_device_t> {
        Ok(wasi::__wasi_device_t::from(dev))
    }

    pub(crate) fn stino_from_nix(ino: libc::ino_t) -> Result<wasi::__wasi_inode_t> {
        Ok(wasi::__wasi_device_t::from(ino))
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_os = "emscripten")] {
        mod emscripten;
        use self::emscripten as imp;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        use self::linux as imp;
    }
}

pub(crate) use imp::filetime;
