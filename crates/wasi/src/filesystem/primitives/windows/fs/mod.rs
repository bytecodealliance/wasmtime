mod create_dir_unchecked;
mod create_file_at_w;
mod dir_entry_inner;
mod dir_options_ext;
mod dir_utils;
mod file_type_ext;
mod get_path;
mod hard_link_unchecked;
mod metadata_ext;
mod oflags;
mod open_impl;
mod open_options_ext;
mod open_unchecked;
mod read_dir_inner;
mod read_link_impl;
mod read_link_unchecked;
mod remove_dir_unchecked;
mod remove_file_unchecked;
mod rename_unchecked;
mod set_times_impl;
mod stat_unchecked;
mod symlink_unchecked;

pub(crate) mod errors;

#[rustfmt::skip]
pub(crate) use crate::filesystem::primitives::{
    via_parent::hard_link as hard_link_impl,
    via_parent::create_dir as create_dir_impl,
    via_parent::rename as rename_impl,
    via_parent::remove_dir as remove_dir_impl,
    manually::stat as stat_impl,
    via_parent::symlink_dir as symlink_dir_impl,
    via_parent::symlink_file as symlink_file_impl,
    via_parent::remove_file as remove_file_impl,
};

pub(crate) use create_dir_unchecked::*;
pub(crate) use dir_entry_inner::*;
pub(crate) use dir_options_ext::*;
pub(crate) use dir_utils::*;
pub(crate) use file_type_ext::*;
pub(crate) use hard_link_unchecked::*;
pub(crate) use metadata_ext::*;
pub(crate) use open_impl::open_impl;
pub(crate) use open_options_ext::*;
pub(crate) use open_unchecked::*;
pub(crate) use read_dir_inner::*;
pub(crate) use read_link_impl::*;
pub(crate) use read_link_unchecked::*;
pub(crate) use remove_dir_unchecked::*;
pub(crate) use remove_file_unchecked::*;
pub(crate) use rename_unchecked::*;
pub(crate) use set_times_impl::*;
pub(crate) use stat_unchecked::*;
pub(crate) use symlink_unchecked::*;

// On Windows, there is a limit of 63 reparse points on any given path.
// <https://docs.microsoft.com/en-us/windows/win32/fileio/reparse-points>
pub(crate) const MAX_SYMLINK_EXPANSIONS: u8 = 63;

pub(crate) fn file_path(file: &std::fs::File) -> Option<std::path::PathBuf> {
    get_path::get_path(file).ok()
}

pub(super) use oflags::*;
