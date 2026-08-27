mod create_dir_unchecked;
mod dir_entry_inner;
#[cfg(not(target_os = "wasi"))]
mod dir_options_ext;
mod dir_utils;
#[cfg(not(any(target_os = "android", target_os = "linux")))]
mod file_path;
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
#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "wasi")))]
mod set_permissions_impl;
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
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
))]
pub(crate) use crate::filesystem::primitives::rustix::darwin::fs::*;
#[cfg(target_os = "freebsd")]
pub(crate) use crate::filesystem::primitives::rustix::freebsd::fs::*;
#[cfg(any(target_os = "android", target_os = "linux"))]
pub(crate) use crate::filesystem::primitives::rustix::linux::fs::*;
#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "freebsd")))]
#[rustfmt::skip]
pub(crate) use crate::filesystem::primitives::{
    manually::open_entry as open_entry_impl,
    manually::open as open_impl,
    manually::stat as stat_impl,
    manually::canonicalize as canonicalize_impl,
    via_parent::set_times_nofollow as set_times_nofollow_impl,
};
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
))]
pub(super) use file_path::file_path_by_ttyname_or_seaching;
#[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
)))]
pub(crate) use file_path::file_path_by_ttyname_or_seaching as file_path;
#[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "wasi"
)))]
pub(crate) use set_permissions_impl::set_permissions_impl;
#[cfg(target_os = "freebsd")]
pub(crate) use set_permissions_impl::set_permissions_impl as set_permissions_manually;
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

/// Test that `file_path` works on a tty path.
#[test]
fn tty_path() {
    #[cfg(unix)]
    use std::os::unix::fs::FileTypeExt;

    let paths: &[&str] = if cfg!(target_os = "freebsd") {
        // On FreeBSD, /dev/{tty,stdin,stdout,stderr} are aliases to different
        // real devices.
        &["/dev/ttyv0", "/dev/pts/0"]
    } else if cfg!(target_os = "illumos") {
        // On illumos, /dev/std{in,out,err} only exist if they're open.
        &["/dev/tty", "/dev/pts/0"]
    } else {
        &["/dev/tty", "/dev/stdin", "/dev/stdout", "/dev/stderr"]
    };

    for path in paths {
        // Not all host configurations have these, so only test them if we can
        // open and canonicalize them, and if they're not FIFOs, which some
        // OS's use for stdin/stdout/stderr.
        if let Ok(file) = std::fs::File::open(path) {
            if !file.metadata().unwrap().file_type().is_fifo() {
                if let Ok(canonical) = std::fs::canonicalize(path) {
                    assert_eq!(
                        file_path(&file)
                            .as_ref()
                            .map(std::fs::canonicalize)
                            .map(Result::unwrap),
                        Some(canonical),
                        "for path {path}, file_path matches canonicalized path"
                    );
                }
            }
        }
    }
}
