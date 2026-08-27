//! Filesystem utilities.

// Allow preexisting warnings that were present in this module before it was
// imported from the cap-primitives crate. These still probably want to get
// resolved in a future change.
#![allow(trivial_numeric_casts)]
#![allow(unsafe_op_in_unsafe_fn)]

mod create_dir;
mod dir_entry;
mod dir_options;
#[cfg(not(any(target_os = "android", target_os = "linux", windows)))]
mod file_path_by_searching;
mod file_type;
mod follow_symlinks;
mod hard_link;
mod maybe_owned_file;
mod metadata;
mod open;
mod open_dir;
mod open_options;
mod open_unchecked_error;
mod read_dir;
mod read_link;
mod remove_dir;
mod remove_file;
mod rename;
mod set_times;
mod stat;
mod symlink;
mod system_time_spec;

pub(crate) mod errors;
pub(crate) mod manually;
pub(crate) mod via_parent;

use maybe_owned_file::MaybeOwnedFile;

#[cfg(not(any(target_os = "android", target_os = "linux", windows)))]
pub(crate) use file_path_by_searching::file_path_by_searching;
pub(crate) use open_unchecked_error::*;

#[cfg(not(windows))]
mod rustix;
#[cfg(not(windows))]
pub(crate) use self::rustix::fs::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use self::windows::fs::*;

pub use create_dir::create_dir;
pub use dir_entry::DirEntry;
pub use dir_options::DirOptions;
#[cfg(windows)]
pub use file_type::_WindowsFileTypeExt;
pub use file_type::FileType;
#[cfg(any(unix, target_os = "vxworks"))]
pub use file_type::FileTypeExt;
pub use follow_symlinks::FollowSymlinks;
pub use hard_link::hard_link;
#[cfg(windows)]
pub use metadata::_WindowsByHandle;
pub use metadata::{Metadata, MetadataExt};
pub use open::open;
pub use open_dir::*;
pub use open_options::*;
pub use read_dir::read_base_dir;
pub use read_link::read_link;
pub use remove_dir::remove_dir;
pub use remove_file::remove_file;
pub use rename::rename;
pub use set_times::{set_times, set_times_nofollow};
pub use stat::stat;
#[cfg(not(windows))]
pub use symlink::symlink;
#[cfg(windows)]
pub use symlink::{symlink_dir, symlink_file};
pub use system_time_spec::SystemTimeSpec;

/// Test that `file_path` works on a few miscellaneous directory paths.
#[test]
fn dir_paths() {
    use crate::ambient_authority;

    for path in [std::env::current_dir().unwrap(), std::env::temp_dir()] {
        let dir = open_ambient_dir(&path, ambient_authority()).unwrap();
        assert_eq!(
            file_path(&dir)
                .as_ref()
                .map(std::fs::canonicalize)
                .map(Result::unwrap),
            Some(std::fs::canonicalize(path).unwrap())
        );
    }
}
