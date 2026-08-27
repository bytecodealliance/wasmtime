mod create_dir_unchecked;
mod dir_entry_inner;
#[cfg(not(target_os = "wasi"))]
mod dir_options_ext;
mod dir_utils;
mod file_type_ext;
mod hard_link_unchecked;
mod is_same_file;
mod metadata_ext;
mod oflags;
mod open_options_ext;
mod open_unchecked;
mod read_dir_inner;
mod read_link_unchecked;
mod remove_dir_unchecked;
mod remove_file_unchecked;
mod rename_unchecked;
#[cfg(not(any(target_os = "android", target_os = "linux")))]
mod set_times_impl;
mod stat_unchecked;
mod symlink_unchecked;
mod times;

pub(crate) mod errors;

// On Linux, use optimized implementations based on
// `openat2` and `O_PATH` when available.
//
// On FreeBSD, use optimized implementations based on
// `O_RESOLVE_BENEATH`/`AT_RESOLVE_BENEATH` and `O_PATH` when available.
#[cfg(target_os = "freebsd")]
pub(crate) use crate::filesystem::primitives::rustix::freebsd::fs::*;
#[cfg(any(target_os = "android", target_os = "linux"))]
pub(crate) use crate::filesystem::primitives::rustix::linux::fs::*;
#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "freebsd")))]
#[rustfmt::skip]
pub(crate) use crate::filesystem::primitives::{
    manually::open as open_impl,
    manually::stat as stat_impl,
    via_parent::set_times_nofollow as set_times_nofollow_impl,
};
#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "freebsd")))]
pub(crate) use set_times_impl::set_times_impl;
#[cfg(target_os = "freebsd")]
pub(crate) use set_times_impl::set_times_impl as set_times_manually;
#[rustfmt::skip]
pub(crate) use crate::filesystem::primitives::{
    via_parent::hard_link as hard_link_impl,
    via_parent::create_dir as create_dir_impl,
    via_parent::read_link as read_link_impl,
    via_parent::rename as rename_impl,
    via_parent::symlink as symlink_impl,
};
#[cfg(not(target_os = "freebsd"))]
#[rustfmt::skip]
pub(crate) use crate::filesystem::primitives::{
    via_parent::remove_dir as remove_dir_impl,
    via_parent::remove_file as remove_file_impl,
};

pub(crate) use create_dir_unchecked::create_dir_unchecked;
pub(crate) use dir_entry_inner::DirEntryInner;
#[cfg(not(target_os = "wasi"))]
pub(crate) use dir_options_ext::DirOptionsExt;
pub(crate) use dir_utils::*;
pub(crate) use file_type_ext::ImplFileTypeExt;
pub(crate) use hard_link_unchecked::hard_link_unchecked;
#[allow(unused_imports)]
pub(crate) use is_same_file::{is_different_file, is_different_file_metadata, is_same_file};
pub(crate) use metadata_ext::ImplMetadataExt;
pub(crate) use open_options_ext::ImplOpenOptionsExt;
pub(crate) use open_unchecked::open_unchecked;
pub(crate) use read_dir_inner::ReadDirInner;
pub(crate) use read_link_unchecked::read_link_unchecked;
pub(crate) use remove_dir_unchecked::remove_dir_unchecked;
pub(crate) use remove_file_unchecked::remove_file_unchecked;
pub(crate) use rename_unchecked::rename_unchecked;
pub(crate) use stat_unchecked::stat_unchecked;
pub(crate) use symlink_unchecked::symlink_unchecked;
#[allow(unused_imports)]
pub(crate) use times::{set_times_follow_unchecked, set_times_nofollow_unchecked, to_timespec};

// On Linux, there is a limit of 40 symlink expansions.
// Source: <https://man7.org/linux/man-pages/man7/path_resolution.7.html>
pub(crate) const MAX_SYMLINK_EXPANSIONS: u8 = 40;

pub(super) use oflags::*;
