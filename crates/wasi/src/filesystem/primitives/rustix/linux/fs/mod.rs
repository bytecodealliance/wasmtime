#[cfg(target_os = "linux")]
mod file_metadata;
mod open_impl;
mod procfs;
mod set_times_impl;
#[cfg(target_os = "linux")]
mod stat_impl;

#[cfg(target_os = "android")]
pub(crate) use crate::filesystem::primitives::manually::stat as stat_impl;
pub(crate) use crate::filesystem::primitives::via_parent::set_times_nofollow as set_times_nofollow_impl;
#[cfg(target_os = "linux")]
pub(crate) use open_impl::open_beneath;
pub(crate) use open_impl::open_impl;
pub(crate) use set_times_impl::set_times_impl;
#[cfg(target_os = "linux")]
pub(crate) use stat_impl::stat_impl;

// In theory we could optimize `link` using `openat2` with `O_PATH` and
// `linkat` with `AT_EMPTY_PATH`, however that requires `CAP_DAC_READ_SEARCH`,
// so it isn't very widely applicable.
