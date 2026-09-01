use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_supported;
use crate::filesystem::primitives::{Metadata, set_times, set_times_nofollow};
use std::path::Path;
use std::time::SystemTime;

fn modified_time(meta: Metadata) -> SystemTime {
    meta.modified().unwrap()
}

#[test]
fn basic_times() {
    let test_symlinks = symlink_supported();

    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create(&dir, "file"));
    check!(h::create_dir(&dir, "dir"));
    if test_symlinks {
        check!(h::symlink_file(&dir, "file", "file_symlink_file"));
        check!(h::symlink_dir(&dir, "dir", "dir_symlink_dir"));
    }

    let file_time = SystemTime::UNIX_EPOCH;
    check!(set_times(&dir, Path::new("file"), None, Some(file_time)));
    assert_eq!(modified_time(check!(h::metadata(&dir, "file"))), file_time);
    if test_symlinks {
        assert_eq!(
            modified_time(check!(h::metadata(&dir, "file_symlink_file"))),
            file_time
        );
    }

    let dir_time = SystemTime::UNIX_EPOCH;
    check!(set_times(&dir, Path::new("dir"), None, Some(dir_time)));
    assert_eq!(modified_time(check!(h::metadata(&dir, "dir"))), dir_time);
    if test_symlinks {
        assert_eq!(
            modified_time(check!(h::metadata(&dir, "dir_symlink_dir"))),
            dir_time
        );
    }
}

#[test]
fn symlink_times() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create(&dir, "file"));
    check!(h::create_dir(&dir, "dir"));
    check!(h::symlink_file(&dir, "file", "file_symlink_file"));
    check!(h::symlink_dir(&dir, "dir", "dir_symlink_dir"));

    let file_time = SystemTime::UNIX_EPOCH;
    check!(set_times(
        &dir,
        Path::new("file_symlink_file"),
        None,
        Some(file_time)
    ));
    assert_eq!(modified_time(check!(h::metadata(&dir, "file"))), file_time);
    assert_eq!(
        modified_time(check!(h::metadata(&dir, "file_symlink_file"))),
        file_time
    );
    assert_eq!(
        modified_time(check!(h::symlink_metadata(&dir, "file"))),
        file_time
    );
    assert_ne!(
        modified_time(check!(h::symlink_metadata(&dir, "file_symlink_file"))),
        file_time
    );

    let dir_time = SystemTime::UNIX_EPOCH;
    check!(set_times(
        &dir,
        Path::new("dir_symlink_dir"),
        None,
        Some(file_time)
    ));
    assert_eq!(modified_time(check!(h::metadata(&dir, "dir"))), dir_time);
    assert_eq!(
        modified_time(check!(h::metadata(&dir, "dir_symlink_dir"))),
        dir_time
    );
    assert_eq!(
        modified_time(check!(h::symlink_metadata(&dir, "dir"))),
        dir_time
    );
    assert_ne!(
        modified_time(check!(h::symlink_metadata(&dir, "dir_symlink_dir"))),
        dir_time
    );
}

#[test]
fn symlink_itself_times() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create(&dir, "file"));
    check!(h::create_dir(&dir, "dir"));
    check!(h::symlink_file(&dir, "file", "file_symlink_file"));
    check!(h::symlink_dir(&dir, "dir", "dir_symlink_dir"));

    let file_time = SystemTime::UNIX_EPOCH;
    check!(set_times_nofollow(
        &dir,
        Path::new("file_symlink_file"),
        None,
        Some(file_time)
    ));
    assert_ne!(modified_time(check!(h::metadata(&dir, "file"))), file_time);
    assert_ne!(
        modified_time(check!(h::metadata(&dir, "file_symlink_file"))),
        file_time
    );
    assert_ne!(
        modified_time(check!(h::symlink_metadata(&dir, "file"))),
        file_time
    );
    assert_eq!(
        modified_time(check!(h::symlink_metadata(&dir, "file_symlink_file"))),
        file_time
    );

    let dir_time = SystemTime::UNIX_EPOCH;
    check!(set_times_nofollow(
        &dir,
        Path::new("dir_symlink_dir"),
        None,
        Some(file_time)
    ));
    assert_ne!(modified_time(check!(h::metadata(&dir, "dir"))), dir_time);
    assert_ne!(
        modified_time(check!(h::metadata(&dir, "dir_symlink_dir"))),
        dir_time
    );
    assert_ne!(
        modified_time(check!(h::symlink_metadata(&dir, "dir"))),
        dir_time
    );
    assert_eq!(
        modified_time(check!(h::symlink_metadata(&dir, "dir_symlink_dir"))),
        dir_time
    );
}
