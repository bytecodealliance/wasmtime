use super::helpers as h;
use super::sys_common::io::tmpdir;
use crate::filesystem::primitives as p;

use std::path::Path;

/*
#[cfg(not(windows))]
fn rename_path_in_use() -> String {
    rustix::io::Errno::BUSY.into().to_string()
}
#[cfg(windows)]
fn rename_path_in_use() -> String {
    todo!("work out error for rename_path_in_use condition")
}
*/

#[cfg(not(windows))]
fn no_such_file_or_directory() -> String {
    rustix::io::Errno::NOENT.to_string()
}
#[cfg(windows)]
fn no_such_file_or_directory() -> String {
    std::io::Error::from_raw_os_error(windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND as i32)
        .to_string()
}

/* // TODO: Platform-specific error code.
cfg_if::cfg_if! {
    if #[cfg(any(
        target_os = "macos",
        target_os = "netbsd",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "ios",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "visionos",
        target_os = "dragonfly"
    ))] {
        fn rename_file_over_dir() -> String {
            rustix::io::Errno::ISDIR.into().to_string()
        }

        fn rename_file_over_dot() -> String {
            rename_file_over_dir()
        }

        fn rename_dot_over_file() -> String {
            rustix::io::Errno::INVAL.into().to_string()
        }
    } else {
        fn rename_file_over_dir() -> String {
            rustix::io::Errno::NOTEMPTY.into().to_string()
        }

        fn rename_file_over_dot() -> String {
            rename_path_in_use()
        }

        fn rename_dot_over_file() -> String {
            rename_path_in_use()
        }
    }
}
*/

#[test]
#[cfg_attr(windows, ignore)] // TODO: Blocked on error message discrepancies
fn rename_basics() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);

    check!(h::create_dir_all(&dir, "foo/bar"));
    check!(h::create(&dir, "foo/bar/file.txt"));

    check!(p::rename(
        &dir,
        Path::new("foo/bar/file.txt"),
        &dir,
        Path::new("foo/bar/renamed.txt")
    ));
    assert!(!h::exists(&dir, "foo/bar/file.txt"));
    assert!(h::exists(&dir, "foo/bar/renamed.txt"));

    check!(p::rename(
        &dir,
        Path::new("foo/bar/renamed.txt"),
        &dir,
        Path::new("foo/bar/renamed.txt")
    ));
    error_contains!(
        p::rename(
            &dir,
            Path::new("foo/bar/renamed.txt"),
            &dir,
            Path::new("..")
        ),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(
            &dir,
            Path::new("foo/bar/renamed.txt"),
            &dir,
            Path::new("foo/../..")
        ),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(
            &dir,
            Path::new("foo/bar/renamed.txt"),
            &dir,
            Path::new("/tmp")
        ),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(
            &dir,
            Path::new("foo/bar/renamed.txt"),
            &dir,
            Path::new("foo/bar/baz/..")
        ),
        &no_such_file_or_directory()
    );
    /* // TODO: Platform-specific error code.
    error!(
        p::rename(&dir, Path::new("foo/bar/renamed.txt"), &dir, Path::new("foo/bar")),
        &rename_file_over_dir()
    );
    */
    check!(p::rename(
        &dir,
        Path::new("foo/bar"),
        &dir,
        Path::new("foo/bar")
    ));
    check!(p::rename(
        &dir,
        Path::new("foo/bar/renamed.txt"),
        &dir,
        Path::new("file.txt")
    ));
    assert!(!h::exists(&dir, "foo/bar/renamed.txt"));
    assert!(h::exists(&dir, "file.txt"));

    /* // TODO: Platform-specific error code.
    error_contains!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("foo/..")),
        &rename_path_in_use()
    );
    error_contains!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("foo/.")),
        &rename_path_in_use()
    );
    error_contains!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("foo/bar/../..")),
        &rename_path_in_use()
    );
    */
    error_contains!(
        p::rename(
            &dir,
            Path::new("file.txt"),
            &dir,
            Path::new("foo/bar/../../..")
        ),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(
            &dir,
            Path::new("file.txt"),
            &dir,
            Path::new("foo/bar/../../../something")
        ),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("")),
        "No such file"
    );
    error_contains!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("/")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("/.")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("/..")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(&dir, Path::new("/"), &dir, Path::new("nope.txt")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(&dir, Path::new("/.."), &dir, Path::new("nope.txt")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::rename(&dir, Path::new("file.txt/"), &dir, Path::new("nope.txt")),
        "Not a directory"
    );

    /* // TODO: Platform-specific error code.
    error!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new(".")),
        &rename_file_over_dot()
    );
    error!(
        p::rename(&dir, Path::new("file.txt"), &dir, Path::new("..")),
        &rename_path_in_use()
    );
    error!(
        p::rename(&dir, Path::new(".."), &dir, Path::new("nope.txt")),
        &rename_path_in_use()
    );
    error!(
        p::rename(&dir, Path::new("."), &dir, Path::new("nope.txt")),
        &rename_dot_over_file()
    );
    */

    check!(h::create(&dir, "existing.txt"));
    check!(p::rename(
        &dir,
        Path::new("file.txt"),
        &dir,
        Path::new("existing.txt")
    ));
    assert!(!h::exists(&dir, "file.txt"));
    assert!(h::exists(&dir, "existing.txt"));
}
