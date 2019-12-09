pub(crate) mod filetime;

pub(crate) mod host_impl {
    use crate::old::snapshot_0::{wasi, Result};
    use std::convert::TryInto;

    pub(crate) fn stnlink_from_nix(nlink: libc::nlink_t) -> Result<wasi::__wasi_linkcount_t> {
        nlink.try_into().map_err(Into::into)
    }
}
