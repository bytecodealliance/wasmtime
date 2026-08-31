// This file contains tests for `MetadataExt`.
//
// `dev`/`ino`/`nlink` are only on the Unix `MetadataExt`.
#![cfg(unix)]

use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_supported;
use crate::filesystem::primitives::{
    FollowSymlinks, Metadata, MetadataExt, hard_link, open_ambient_dir, stat,
};
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

    // The directory and files inside it should be on the same device.
    assert_eq!(tmpdir_metadata.dev(), a_metadata.dev());
    assert_eq!(a_metadata.dev(), b_metadata.dev());

    // They should all have distinct inodes.
    assert_ne!(tmpdir_metadata.ino(), a_metadata.ino());
    assert_ne!(tmpdir_metadata.ino(), b_metadata.ino());
    assert_ne!(a_metadata.ino(), b_metadata.ino());

    // The files should start with just one link.
    assert_eq!(a_metadata.nlink(), 1);
    assert_eq!(b_metadata.nlink(), 1);

    // Add another link and check for it.
    check!(hard_link(&dir, Path::new("b"), &dir, Path::new("c")));
    let b_metadata = check!(Metadata::from_file(&b));
    assert_eq!(b_metadata.nlink(), 2);

    // Check that the metadata has dev/nlink/ino.
    tmpdir_metadata.dev();
    tmpdir_metadata.nlink();
    tmpdir_metadata.ino();
    a_metadata.dev();
    a_metadata.nlink();
    a_metadata.ino();
    b_metadata.dev();
    b_metadata.nlink();
    b_metadata.ino();
    a_dir_metadata.dev();
    a_dir_metadata.nlink();
    a_dir_metadata.ino();
    b_dir_metadata.dev();
    b_dir_metadata.nlink();
    b_dir_metadata.ino();
    a_symlink_metadata.dev();
    a_symlink_metadata.nlink();
    a_symlink_metadata.ino();
    b_symlink_metadata.dev();
    b_symlink_metadata.nlink();
    b_symlink_metadata.ino();

    if symlink_supported() {
        check!(h::symlink_file(&dir, "b", "d"));
        let d_metadata = check!(stat(&dir, Path::new("d"), FollowSymlinks::Yes));
        let d_symlink_metadata = check!(stat(&dir, Path::new("d"), FollowSymlinks::No));

        d_metadata.dev();
        d_metadata.nlink();
        d_metadata.ino();
        d_symlink_metadata.dev();
        d_symlink_metadata.nlink();
        d_symlink_metadata.ino();

        assert_ne!(
            (d_symlink_metadata.ino(), d_symlink_metadata.dev()),
            (b_metadata.ino(), b_metadata.dev())
        );
        assert_eq!(
            (d_metadata.ino(), d_metadata.dev()),
            (b_metadata.ino(), b_metadata.dev())
        );
    }
}
