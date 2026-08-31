use super::helpers as h;
use super::sys_common::io::tmpdir;
#[allow(unused_imports)]
use super::sys_common::symlink_supported;
use crate::filesystem::primitives as p;

use std::path::Path;

#[test]
fn cap_smoke_test() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));
    check!(h::write(&dir, "red.txt", b"hello world\n"));
    check!(h::write(&dir, "dir/green.txt", b"goodmight moon\n"));
    check!(h::write(&dir, "dir/inner/blue.txt", b"hey mars\n"));

    let inner = check!(p::open_dir(&dir, Path::new("dir/inner")));

    check!(h::open(&dir, "red.txt"));

    #[cfg(not(windows))]
    error!(h::open(&dir, "blue.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "blue.txt"), 2);

    #[cfg(not(windows))]
    error!(h::open(&dir, "green.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "green.txt"), 2);

    check!(h::open(&dir, "./red.txt"));

    #[cfg(not(windows))]
    error!(h::open(&dir, "./blue.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "./blue.txt"), 2);

    #[cfg(not(windows))]
    error!(h::open(&dir, "./green.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "./green.txt"), 2);

    #[cfg(not(windows))]
    error!(h::open(&dir, "dir/red.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "dir/red.txt"), 2);

    check!(h::open(&dir, "dir/green.txt"));

    #[cfg(not(windows))]
    error!(h::open(&dir, "dir/blue.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "dir/blue.txt"), 2);

    #[cfg(not(windows))]
    error!(h::open(&dir, "dir/inner/red.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "dir/inner/red.txt"), 2);

    #[cfg(not(windows))]
    error!(h::open(&dir, "dir/inner/green.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&dir, "dir/inner/green.txt"), 2);

    check!(h::open(&dir, "dir/inner/blue.txt"));

    check!(h::open(&dir, "dir/../red.txt"));
    check!(h::open(&dir, "dir/inner/../../red.txt"));
    check!(h::open(&dir, "dir/inner/../inner/../../red.txt"));

    #[cfg(not(windows))]
    error!(h::open(&inner, "red.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&inner, "red.txt"), 2);

    #[cfg(not(windows))]
    error!(h::open(&inner, "green.txt"), "No such file");
    #[cfg(windows)]
    error!(h::open(&inner, "green.txt"), 2);

    error_contains!(
        h::open(&inner, "../inner/blue.txt"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        h::open(&inner, "../inner/red.txt"),
        "a path led outside of the filesystem"
    );

    #[cfg(not(windows))]
    error!(p::open_dir(&inner, Path::new("")), "No such file");
    #[cfg(windows)]
    error!(p::open_dir(&inner, Path::new("")), 2);

    error_contains!(
        p::open_dir(&inner, Path::new("/")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::open_dir(&inner, Path::new("/etc/services")),
        "a path led outside of the filesystem"
    );
    check!(p::open_dir(&inner, Path::new(".")));
    check!(p::open_dir(&inner, Path::new("./")));
    check!(p::open_dir(&inner, Path::new("./.")));
    error_contains!(
        p::open_dir(&inner, Path::new("..")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::open_dir(&inner, Path::new("../")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::open_dir(&inner, Path::new("../.")),
        "a path led outside of the filesystem"
    );
    error_contains!(
        p::open_dir(&inner, Path::new("./..")),
        "a path led outside of the filesystem"
    );
}

#[test]
fn symlinks() {
    #[cfg(windows)]
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));
    check!(h::write(&dir, "red.txt", b"hello world\n"));
    check!(h::write(&dir, "dir/green.txt", b"goodmight moon\n"));
    check!(h::write(&dir, "dir/inner/blue.txt", b"hey mars\n"));

    let inner = check!(p::open_dir(&dir, Path::new("dir/inner")));

    check!(h::symlink(&dir, "dir", "link"));
    #[cfg(not(windows))]
    check!(h::symlink(&dir, "does_not_exist", "badlink"));

    check!(h::open(&dir, "link/../red.txt"));
    check!(h::open(&dir, "link/green.txt"));
    check!(h::open(&dir, "link/inner/blue.txt"));
    #[cfg(not(windows))]
    {
        error_contains!(h::open(&dir, "link/red.txt"), "No such file");
        error_contains!(h::open(&dir, "link/../green.txt"), "No such file");
    }
    #[cfg(windows)]
    {
        error_contains!(
            h::open(&dir, "link/red.txt"),
            "The system cannot find the file specified."
        );
        error_contains!(
            h::open(&dir, "link/../green.txt"),
            "The system cannot find the file specified."
        );
    }

    check!(h::open(&dir, "./dir/.././/link/..///./red.txt"));
    check!(h::open(&dir, "link/inner/../inner/../../red.txt"));
    error_contains!(
        h::open(&inner, "../inner/../inner/../../link/other.txt"),
        "a path led outside of the filesystem"
    );
    #[cfg(not(windows))]
    {
        error_contains!(
            h::open(&dir, "./dir/.././/link/..///./not.txt"),
            "No such file"
        );
        error_contains!(h::open(&dir, "link/other.txt"), "No such file");
        error_contains!(h::open(&dir, "badlink/../red.txt"), "No such file");
    }
    #[cfg(windows)]
    {
        error_contains!(
            h::open(&dir, "./dir/.././/link/..///./not.txt"),
            "The system cannot find the file specified."
        );
        error_contains!(
            h::open(&dir, "link/other.txt"),
            "The system cannot find the file specified."
        );
    }
}

#[test]
#[cfg(not(windows))]
fn symlink_loop() {
    let tmpdir = tmpdir();

    let dir = h::dir_of(&tmpdir);
    check!(h::symlink(&dir, "link", "link"));
    // TODO: Check the error message
    error_contains!(h::open(&dir, "link"), "");
}

#[test]
fn symlink_loop_from_rename() {
    #[cfg(windows)]
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let dir = h::dir_of(&tmpdir);
    check!(h::create(&dir, "file"));
    check!(h::symlink(&dir, "file", "link"));
    check!(h::open(&dir, "link"));
    check!(p::rename(
        &dir,
        Path::new("file"),
        &dir,
        Path::new("renamed")
    ));
    error_contains!(h::open(&dir, "link"), "");
    check!(p::rename(&dir, Path::new("link"), &dir, Path::new("file")));
    error_contains!(h::open(&dir, "file"), "");
    check!(p::rename(&dir, Path::new("file"), &dir, Path::new("link")));
    error_contains!(h::open(&dir, "link"), "");
    check!(p::rename(
        &dir,
        Path::new("renamed"),
        &dir,
        Path::new("file")
    ));
    check!(h::open(&dir, "link"));
}

#[cfg(target_os = "linux")]
#[test]
fn proc_self_fd() {
    let dir = check!(std::fs::File::open("/proc/self/fd"));
    // This should fail with "too many levels of symbolic links".
    h::open(&dir, "0").unwrap_err();
}
