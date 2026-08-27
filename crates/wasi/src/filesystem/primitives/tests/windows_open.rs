//! On Windows, cap-std uses the technique of looking up absolute paths for
//! directory handles. This would be racy, except that cap-std uses Windows'
//! sharing modes to prevent open directories from being removed or renamed.
//! Test that this works.

#![cfg(windows)]

use super::helpers as h;
use super::sys_common::io::tmpdir;
use crate::filesystem::primitives as p;
use std::path::Path;

#[test]
fn windows_open_one() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "aaa"));

    let dir = check!(p::open_dir(&start, Path::new("aaa")));

    // Attempts to remove or rename the open directory should fail.
    p::remove_dir(&start, Path::new("aaa")).unwrap_err();
    p::rename(&start, Path::new("aaa"), &start, Path::new("zzz")).unwrap_err();

    drop(dir);

    // Now that we've dropped the handle, the same operations should succeed.
    check!(p::rename(
        &start,
        Path::new("aaa"),
        &start,
        Path::new("xxx")
    ));
    check!(p::remove_dir(&start, Path::new("xxx")));
}

#[test]
fn windows_open_multiple() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&start, "aaa/bbb"));

    let dir = check!(p::open_dir(&start, Path::new("aaa/bbb")));

    // Attempts to remove or rename any component of the open directory should
    // fail.
    p::remove_dir(&start, Path::new("aaa/bbb")).unwrap_err();
    p::remove_dir(&start, Path::new("aaa")).unwrap_err();
    p::rename(&start, Path::new("aaa/bbb"), &start, Path::new("aaa/yyy")).unwrap_err();
    p::rename(&start, Path::new("aaa"), &start, Path::new("zzz")).unwrap_err();

    drop(dir);

    // Now that we've dropped the handle, the same operations should succeed.
    check!(p::rename(
        &start,
        Path::new("aaa/bbb"),
        &start,
        Path::new("aaa/www")
    ));
    check!(p::rename(
        &start,
        Path::new("aaa"),
        &start,
        Path::new("xxx")
    ));
    check!(p::remove_dir(&start, Path::new("xxx/www")));
    check!(p::remove_dir(&start, Path::new("xxx")));
}

/// Like `windows_open_multiple`, but does so within a directory that we
/// can close and then independently mutate.
#[test]
fn windows_open_tricky() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "qqq"));

    let qqq = check!(p::open_dir(&start, Path::new("qqq")));
    check!(h::create_dir_all(&qqq, "aaa/bbb"));

    let dir = check!(p::open_dir(&qqq, Path::new("aaa/bbb")));

    // Now drop `qqq`.
    drop(qqq);

    // Attempts to remove or rename any component of the open directory should
    // fail.
    p::remove_dir(&dir, Path::new("aaa/bbb")).unwrap_err();
    p::remove_dir(&dir, Path::new("aaa")).unwrap_err();
    p::rename(&dir, Path::new("aaa/bbb"), &dir, Path::new("aaa/yyy")).unwrap_err();
    p::rename(&dir, Path::new("aaa"), &dir, Path::new("zzz")).unwrap_err();
    p::remove_dir(&start, Path::new("qqq/aaa/bbb")).unwrap_err();
    p::remove_dir(&start, Path::new("qqq/aaa")).unwrap_err();
    p::remove_dir(&start, Path::new("qqq")).unwrap_err();
    p::rename(
        &dir,
        Path::new("qqq/aaa/bbb"),
        &dir,
        Path::new("qqq/aaa/yyy"),
    )
    .unwrap_err();
    p::rename(&start, Path::new("qqq/aaa"), &start, Path::new("qqq/zzz")).unwrap_err();
    p::rename(&start, Path::new("qqq"), &start, Path::new("vvv")).unwrap_err();

    drop(dir);

    // Now that we've dropped the handle, the same operations should succeed.
    check!(p::rename(
        &start,
        Path::new("qqq/aaa/bbb"),
        &start,
        Path::new("qqq/aaa/www")
    ));
    check!(p::rename(
        &start,
        Path::new("qqq/aaa"),
        &start,
        Path::new("qqq/xxx")
    ));
    check!(p::rename(
        &start,
        Path::new("qqq"),
        &start,
        Path::new("uuu")
    ));
    check!(p::remove_dir(&start, Path::new("uuu/xxx/www")));
    check!(p::remove_dir(&start, Path::new("uuu/xxx")));
    check!(p::remove_dir(&start, Path::new("uuu")));
}

/// Like `windows_open_one` but uses `open_ambient_dir` instead of `open_dir`.
#[test]
fn windows_open_ambient() {
    let ambient_dir = tempfile::tempdir().unwrap();

    let start = check!(h::open_ambient_dir(ambient_dir.path()));
    check!(h::create_dir(&start, "aaa"));

    let dir = check!(h::open_ambient_dir(ambient_dir.path().join("aaa")));

    // Attempts to remove or rename the open directory should fail.
    p::remove_dir(&start, Path::new("aaa")).unwrap_err();
    p::rename(&start, Path::new("aaa"), &start, Path::new("zzz")).unwrap_err();

    drop(dir);

    // Now that we've dropped the handle, the same operations should succeed.
    check!(p::rename(
        &start,
        Path::new("aaa"),
        &start,
        Path::new("xxx")
    ));
    check!(p::remove_dir(&start, Path::new("xxx")));
}

#[test]
fn windows_open_special() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    // Opening any of these should fail.
    for device in &[
        "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
        "LPT6", "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
    ] {
        for suffix in &[
            "",
            " ",
            ".",
            ". ",
            ".ext",
            ".ext.",
            ".ext. ",
            ".ext ",
            ".ext.more",
            ".ext.more.",
            ".ext.more ",
            ".ext.more. ",
            ".ext.more .",
        ] {
            let name = format!("{}{}", device, suffix);
            eprintln!("testing '{}'", name);

            match h::open(&start, &name).unwrap_err().kind() {
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {}
                kind => panic!("unexpected error: {:?}", kind),
            }

            let mut options = p::OpenOptions::new();
            options.write(true);
            match p::open(&start, Path::new(&name), &options)
                .unwrap_err()
                .kind()
            {
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {}
                kind => panic!("unexpected error: {:?}", kind),
            }

            match h::create(&start, &name).unwrap_err().kind() {
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {}
                kind => panic!("unexpected error: {:?}", kind),
            }
        }
    }
}
