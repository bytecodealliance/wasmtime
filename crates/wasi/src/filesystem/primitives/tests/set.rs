// Test `set_permissions` and `set_times`.

#[macro_use]
mod sys_common;

use std::io;
use std::path::Path;
use sys_common::io::{tmpdir, TempDir};
use sys_common::symlink_supported;

#[cfg(not(windows))]
fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, tmpdir: &TempDir, dst: Q) -> io::Result<()> {
    tmpdir.symlink(src, dst)
}
#[cfg(not(windows))]
fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    tmpdir: &TempDir,
    dst: Q,
) -> io::Result<()> {
    tmpdir.symlink(src, dst)
}
#[cfg(windows)]
fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(src: P, tmpdir: &TempDir, dst: Q) -> io::Result<()> {
    tmpdir.symlink_dir(src, dst)
}
#[cfg(windows)]
fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    tmpdir: &TempDir,
    dst: Q,
) -> io::Result<()> {
    tmpdir.symlink_file(src, dst)
}

#[test]
fn basic_perms() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();
    check!(tmpdir.create("file"));
    check!(tmpdir.create_dir("dir"));
    check!(symlink_file("file", &tmpdir, "symlink_file"));
    check!(symlink_dir("dir", &tmpdir, "symlink_dir"));

    let mut file_perms = check!(tmpdir.metadata("file")).permissions();
    assert!(!file_perms.readonly());
    file_perms.set_readonly(true);
    check!(tmpdir.set_permissions("file", file_perms.clone()));
    assert_eq!(
        check!(tmpdir.metadata("file")).permissions(),
        file_perms.clone()
    );
    assert_eq!(
        check!(tmpdir.metadata("symlink_file")).permissions(),
        file_perms
    );

    let mut dir_perms = check!(tmpdir.metadata("dir")).permissions();
    assert!(!dir_perms.readonly());
    dir_perms.set_readonly(true);
    check!(tmpdir.set_permissions("dir", dir_perms.clone()));
    assert_eq!(
        check!(tmpdir.metadata("dir")).permissions(),
        dir_perms.clone()
    );
    assert_eq!(
        check!(tmpdir.metadata("symlink_dir")).permissions(),
        dir_perms
    );
}

#[test]
fn symlink_perms() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();
    check!(tmpdir.create("file"));
    check!(tmpdir.create_dir("dir"));
    check!(symlink_file("file", &tmpdir, "symlink_file"));
    check!(symlink_dir("dir", &tmpdir, "symlink_dir"));

    let mut file_perms = check!(tmpdir.metadata("symlink_file")).permissions();
    assert!(!file_perms.readonly());
    file_perms.set_readonly(true);
    check!(tmpdir.set_permissions("symlink_file", file_perms.clone()));

    // On Windows, `set_permissions` does not follow symlinks. On non-Windows,
    // it does. See https://github.com/rust-lang/rust/issues/75942 for discussion.
    if cfg!(windows) {
        assert_eq!(
            check!(tmpdir.symlink_metadata("symlink_file")).permissions(),
            file_perms
        );
    } else {
        assert_eq!(
            check!(tmpdir.metadata("file")).permissions(),
            file_perms.clone()
        );
        assert_eq!(
            check!(tmpdir.metadata("symlink_file")).permissions(),
            file_perms
        );
    }

    let mut dir_perms = check!(tmpdir.metadata("symlink_dir")).permissions();
    assert!(!dir_perms.readonly());
    dir_perms.set_readonly(true);
    check!(tmpdir.set_permissions("symlink_dir", dir_perms.clone()));

    // On Windows, `set_permissions` does not follow symlinks. On non-Windows,
    // it does. See https://github.com/rust-lang/rust/issues/75942 for discussion.
    if cfg!(windows) {
        assert_eq!(
            check!(tmpdir.symlink_metadata("symlink_dir")).permissions(),
            dir_perms
        );
    } else {
        assert_eq!(
            check!(tmpdir.metadata("dir")).permissions(),
            dir_perms.clone()
        );
        assert_eq!(
            check!(tmpdir.metadata("symlink_dir")).permissions(),
            dir_perms
        );
    }
}
