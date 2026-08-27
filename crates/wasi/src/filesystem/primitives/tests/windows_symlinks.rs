#![cfg(windows)]

use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_supported;
use crate::filesystem::primitives as p;

use std::path::Path;

#[test]
fn windows_symlinks() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);

    check!(h::create(&start, "file"));
    check!(h::create_dir(&start, "dir"));

    // Windows lets these succeed.
    check!(h::symlink_dir(&start, "file", "file_symlink_dir"));
    check!(h::symlink_file(&start, "dir", "dir_symlink_file"));

    // But accessing them fails.
    assert!(h::open(&start, "dir_symlink_file").is_err());
    assert!(h::open(&start, "file_symlink_dir").is_err());
    assert!(p::open_dir(&start, Path::new("dir_symlink_file")).is_err());
    assert!(p::open_dir(&start, Path::new("file_symlink_dir")).is_err());
    assert!(h::metadata(&start, "dir_symlink_file").is_err());
    assert!(h::metadata(&start, "file_symlink_dir").is_err());

    assert!(
        check!(h::symlink_metadata(&start, "file_symlink_dir"))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(h::symlink_metadata(&start, "dir_symlink_file"))
            .file_type()
            .is_symlink()
    );
}

#[test]
fn windows_symlinks_ambient() {
    use std::fs;
    use std::os::windows::fs::{symlink_dir, symlink_file};

    if !symlink_supported() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();

    check!(fs::File::create(dir.path().join("file")));
    check!(fs::create_dir(dir.path().join("dir")));

    // Windows lets these succeed.
    check!(symlink_dir("file", dir.path().join("file_symlink_dir")));
    check!(symlink_file("dir", dir.path().join("dir_symlink_file")));

    // But accessing them fails.
    assert!(fs::File::open(dir.path().join("dir_symlink_file")).is_err());
    assert!(fs::File::open(dir.path().join("file_symlink_dir")).is_err());

    assert!(h::open_ambient_dir(dir.path().join("dir_symlink_file")).is_err());
    assert!(h::open_ambient_dir(dir.path().join("file_symlink_dir")).is_err());
    assert!(fs::metadata(dir.path().join("dir_symlink_file")).is_err());
    assert!(fs::metadata(dir.path().join("file_symlink_dir")).is_err());

    assert!(
        check!(fs::symlink_metadata(dir.path().join("file_symlink_dir")))
            .file_type()
            .is_symlink()
    );
    assert!(
        check!(fs::symlink_metadata(dir.path().join("dir_symlink_file")))
            .file_type()
            .is_symlink()
    );
}
