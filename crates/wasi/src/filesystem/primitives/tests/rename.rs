#[macro_use]
mod sys_common;

use sys_common::io::tmpdir;

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

    check!(tmpdir.create_dir_all("foo/bar"));
    check!(tmpdir.create("foo/bar/file.txt"));

    check!(tmpdir.rename("foo/bar/file.txt", &tmpdir, "foo/bar/renamed.txt"));
    assert!(!tmpdir.exists("foo/bar/file.txt"));
    assert!(tmpdir.exists("foo/bar/renamed.txt"));

    check!(tmpdir.rename("foo/bar/renamed.txt", &tmpdir, "foo/bar/renamed.txt"));
    error_contains!(
        tmpdir.rename("foo/bar/renamed.txt", &tmpdir, ".."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("foo/bar/renamed.txt", &tmpdir, "foo/../.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("foo/bar/renamed.txt", &tmpdir, "/tmp"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("foo/bar/renamed.txt", &tmpdir, "foo/bar/baz/.."),
        &no_such_file_or_directory()
    );
    /* // TODO: Platform-specific error code.
    error!(
        tmpdir.rename("foo/bar/renamed.txt", &tmpdir, "foo/bar"),
        &rename_file_over_dir()
    );
    */
    check!(tmpdir.rename("foo/bar", &tmpdir, "foo/bar"));
    check!(tmpdir.rename("foo/bar/renamed.txt", &tmpdir, "file.txt"));
    assert!(!tmpdir.exists("foo/bar/renamed.txt"));
    assert!(tmpdir.exists("file.txt"));

    /* // TODO: Platform-specific error code.
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "foo/.."),
        &rename_path_in_use()
    );
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "foo/."),
        &rename_path_in_use()
    );
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "foo/bar/../.."),
        &rename_path_in_use()
    );
    */
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "foo/bar/../../.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "foo/bar/../../../something"),
        "a path led outside of the filesystem"
    );
    error_contains!(tmpdir.rename("file.txt", &tmpdir, ""), "No such file");
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "/"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "/."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("file.txt", &tmpdir, "/.."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("/", &tmpdir, "nope.txt"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("/..", &tmpdir, "nope.txt"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        tmpdir.rename("file.txt/", &tmpdir, "nope.txt"),
        "Not a directory"
    );

    /* // TODO: Platform-specific error code.
    error!(
        tmpdir.rename("file.txt", &tmpdir, "."),
        &rename_file_over_dot()
    );
    error!(
        tmpdir.rename("file.txt", &tmpdir, ".."),
        &rename_path_in_use()
    );
    error!(
        tmpdir.rename("..", &tmpdir, "nope.txt"),
        &rename_path_in_use()
    );
    error!(
        tmpdir.rename(".", &tmpdir, "nope.txt"),
        &rename_dot_over_file()
    );
    */

    check!(tmpdir.create("existing.txt"));
    check!(tmpdir.rename("file.txt", &tmpdir, "existing.txt"));
    assert!(!tmpdir.exists("file.txt"));
    assert!(tmpdir.exists("existing.txt"));
}
