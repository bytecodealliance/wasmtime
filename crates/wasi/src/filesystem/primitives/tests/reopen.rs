#![cfg(not(target_os = "freebsd"))] // FreeBSD can't reopen arbitrary files.

#[macro_use]
mod sys_common;

use cap_fs_ext::{OpenOptions, Reopen};
use std::io::{Read, Write};
use sys_common::io::tmpdir;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_READ;

#[test]
fn basic_reopen() {
    let tmpdir = tmpdir();

    // Open the file with both read and write privileges, so that we can
    // write to it, and then reopen it for reading.
    let mut file = check!(tmpdir.open_with(
        "file",
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
    ));
    check!(file.write_all(b"hello, world"));

    let mut buf = String::new();
    check!(file.read_to_string(&mut buf));
    assert!(buf.is_empty());

    let mut ro = check!(file.reopen(OpenOptions::new().read(true)));
    let mut another = check!(file.reopen(OpenOptions::new().read(true)));
    check!(ro.read_to_string(&mut buf));
    assert_eq!(buf, "hello, world");
    buf.clear();
    check!(another.read_to_string(&mut buf));
    assert_eq!(buf, "hello, world");
    buf.clear();
    check!(another.read_to_string(&mut buf));
    assert!(buf.is_empty());
}

/// Don't allow `reopen` to grant new permissions.
#[test]
fn reopen_perms() {
    let tmpdir = tmpdir();

    let mut file = check!(tmpdir.create("file"));
    check!(file.write_all(b"hello, world"));

    let mut buf = String::new();
    assert!(file.read_to_string(&mut buf).is_err());

    // `file` is write-only, so can't re-open for reading.
    assert!(file.reopen(OpenOptions::new().read(true)).is_err());

    // `ro` is read-only, so can't re-open for writing.
    let mut ro = check!(tmpdir.open("file"));
    assert!(ro.write_all(b"hello, world").is_err());
    assert!(ro.reopen(OpenOptions::new().write(true)).is_err());
    check!(ro.read_to_string(&mut buf));
    assert_eq!(buf, "hello, world");
}

/// Test reopen using `access_mode` explicitly.
#[cfg(windows)]
#[test]
fn reopen_explicit_access() {
    use cap_std::fs::OpenOptionsExt;

    let tmpdir = tmpdir();

    // Open the file with both read and write privileges, so that we can
    // write to it, and then reopen it for reading.
    let mut file = check!(tmpdir.open_with(
        "file",
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
    ));
    check!(file.write_all(b"hello, world"));

    let mut buf = String::new();
    check!(file.read_to_string(&mut buf));
    assert!(buf.is_empty());

    let mut ro = check!(file.reopen(OpenOptions::new().access_mode(FILE_GENERIC_READ)));
    check!(ro.read_to_string(&mut buf));
    assert_eq!(buf, "hello, world");
}
