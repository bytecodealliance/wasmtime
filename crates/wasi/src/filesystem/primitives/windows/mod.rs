use crate::filesystem::primitives::Metadata;
use std::fs;
use std::io;
use std::os::windows::fs::MetadataExt;
use std::path::Path;

mod create_dir_unchecked;
mod create_file_at_w;
mod dir_utils;
mod get_path;
mod hard_link_unchecked;
mod oflags;
mod open_fast;
mod open_options_ext;
mod open_unchecked;
mod read_dir;
mod read_link;
mod read_link_unchecked;
mod remove_dir_unchecked;
mod remove_file_unchecked;
mod rename_unchecked;
mod set_times;
mod stat_unchecked;
mod symlink_unchecked;

pub(crate) mod errors;

pub(crate) use create_dir_unchecked::*;
pub(crate) use dir_utils::*;
pub(crate) use hard_link_unchecked::*;
pub(crate) use open_fast::*;
pub(crate) use open_options_ext::*;
pub(crate) use open_unchecked::*;
pub(crate) use read_dir::*;
pub(crate) use read_link::*;
pub(crate) use read_link_unchecked::*;
pub(crate) use remove_dir_unchecked::*;
pub(crate) use remove_file_unchecked::*;
pub(crate) use rename_unchecked::*;
pub(crate) use set_times::*;
pub(crate) use stat_unchecked::*;
pub(crate) use symlink_unchecked::*;

// On Windows, there is a limit of 63 reparse points on any given path.
// <https://docs.microsoft.com/en-us/windows/win32/fileio/reparse-points>
pub(crate) const MAX_SYMLINK_EXPANSIONS: u8 = 63;

/// Returns an error if `path` contains a `:` anywhere.
///
/// This must be called on every path that is resolved relative to a
/// directory handle before it's handed to the OS.
///
/// On Windows `:` is disallowed in file names, and a `:` is either part of a
/// drive prefix, as in `C:` or `C:foo`, or names an NTFS alternate data stream,
/// as in `file:stream`. Rust's `Path::components` parsing of `C:foo` also
/// depends on where it is in the `Path`, for example it's a drive prefix in
/// `C:foo` but it's a normal component in `./C:foo`. The `primitives` module in
/// this crate generally assumes that a `Normal` path component parsed alone
/// will again yield a `Normal` path component, but Windows drive prefixes break
/// this assumption.
///
/// Given all this the most conservative thing to do is to simply disallow `:`
/// everywhere on Windows. It's already not allowed in filenames and otherwise
/// handling it would require extremely careful management of parsing and
/// ensuring that just because a path component is `Normal` once doesn't mean
/// it'll re-parse as `Normal`.
pub(crate) fn check_no_colon(path: &Path) -> io::Result<()> {
    if path.as_os_str().as_encoded_bytes().contains(&b':') {
        return Err(errors::invalid_name());
    }
    Ok(())
}

pub(super) use oflags::*;

impl Metadata {
    pub(super) fn file_attributes(&self) -> u32 {
        let Metadata::Std(std) = self;
        std.file_attributes()
    }
}

/// Open a directory by performing an unsandboxed `openat`-like operation.
#[inline]
pub(super) fn open_dir_unchecked(start: &fs::File, path: &Path) -> io::Result<fs::File> {
    open_unchecked(start, path, &dir_options()).map_err(Into::into)
}
