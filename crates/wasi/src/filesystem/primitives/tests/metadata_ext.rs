// This file contains tests for `Metadata::{identity, nlink}`.
#![cfg(unix)]

use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_supported;
use crate::filesystem::primitives::{FollowSymlinks, Metadata, hard_link, open_ambient_dir, stat};
use std::path::Path;

#[test]
fn test_metadata_ext() {
    let tmpdir = tmpdir();
    let dir = check!(open_ambient_dir(tmpdir.path(),));
    let a = check!(h::create(&dir, "a"));
    let b = check!(h::create(&dir, "b"));
    let tmpdir_metadata = check!(Metadata::from_file(&dir));
    let a_metadata = check!(Metadata::from_file(&a));
    let b_metadata = check!(Metadata::from_file(&b));
    let a_dir_metadata = check!(stat(&dir, Path::new("a"), FollowSymlinks::Yes));
    let b_dir_metadata = check!(stat(&dir, Path::new("b"), FollowSymlinks::Yes));
    let a_symlink_metadata = check!(stat(&dir, Path::new("a"), FollowSymlinks::No));
    let b_symlink_metadata = check!(stat(&dir, Path::new("b"), FollowSymlinks::No));

    // They should all be distinct files.
    assert_ne!(tmpdir_metadata.identity(), a_metadata.identity());
    assert_ne!(tmpdir_metadata.identity(), b_metadata.identity());
    assert_ne!(a_metadata.identity(), b_metadata.identity());

    // Metadata from a file and from a path to it should agree.
    assert_eq!(a_metadata.identity(), a_dir_metadata.identity());
    assert_eq!(b_metadata.identity(), b_dir_metadata.identity());
    assert_eq!(a_metadata.identity(), a_symlink_metadata.identity());
    assert_eq!(b_metadata.identity(), b_symlink_metadata.identity());

    // The files should start with just one link.
    assert_eq!(a_metadata.nlink(), 1);
    assert_eq!(b_metadata.nlink(), 1);
    assert_eq!(a_dir_metadata.nlink(), 1);
    assert_eq!(b_symlink_metadata.nlink(), 1);

    // Add another link and check for it.
    check!(hard_link(&dir, Path::new("b"), &dir, Path::new("c")));
    let b_metadata = check!(Metadata::from_file(&b));
    assert_eq!(b_metadata.nlink(), 2);

    if symlink_supported() {
        check!(h::symlink(&dir, "b", "d"));
        let d_metadata = check!(stat(&dir, Path::new("d"), FollowSymlinks::Yes));
        let d_symlink_metadata = check!(stat(&dir, Path::new("d"), FollowSymlinks::No));

        assert_ne!(d_symlink_metadata.identity(), b_metadata.identity());
        assert_eq!(d_metadata.identity(), b_metadata.identity());
        assert_eq!(d_metadata.nlink(), 2);
        assert_eq!(d_symlink_metadata.nlink(), 1);
    }
}
