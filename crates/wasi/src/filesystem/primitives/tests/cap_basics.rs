#[macro_use]
mod sys_common;

use sys_common::io::tmpdir;
use sys_common::symlink_supported;

#[test]
fn cap_smoke_test() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));
    check!(tmpdir.write("red.txt", b"hello world\n"));
    check!(tmpdir.write("dir/green.txt", b"goodmight moon\n"));
    check!(tmpdir.write("dir/inner/blue.txt", b"hey mars\n"));

    let inner = check!(tmpdir.open_dir("dir/inner"));

    check!(tmpdir.open("red.txt"));

    #[cfg(not(windows))]
    error!(tmpdir.open("blue.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("blue.txt"), 2);

    #[cfg(not(windows))]
    error!(tmpdir.open("green.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("green.txt"), 2);

    check!(tmpdir.open("./red.txt"));

    #[cfg(not(windows))]
    error!(tmpdir.open("./blue.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("./blue.txt"), 2);

    #[cfg(not(windows))]
    error!(tmpdir.open("./green.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("./green.txt"), 2);

    #[cfg(not(windows))]
    error!(tmpdir.open("dir/red.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("dir/red.txt"), 2);

    check!(tmpdir.open("dir/green.txt"));

    #[cfg(not(windows))]
    error!(tmpdir.open("dir/blue.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("dir/blue.txt"), 2);

    #[cfg(not(windows))]
    error!(tmpdir.open("dir/inner/red.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("dir/inner/red.txt"), 2);

    #[cfg(not(windows))]
    error!(tmpdir.open("dir/inner/green.txt"), "No such file");
    #[cfg(windows)]
    error!(tmpdir.open("dir/inner/green.txt"), 2);

    check!(tmpdir.open("dir/inner/blue.txt"));

    check!(tmpdir.open("dir/../red.txt"));
    check!(tmpdir.open("dir/inner/../../red.txt"));
    check!(tmpdir.open("dir/inner/../inner/../../red.txt"));

    #[cfg(not(windows))]
    error!(inner.open("red.txt"), "No such file");
    #[cfg(windows)]
    error!(inner.open("red.txt"), 2);

    #[cfg(not(windows))]
    error!(inner.open("green.txt"), "No such file");
    #[cfg(windows)]
    error!(inner.open("green.txt"), 2);

    error_contains!(
        inner.open("../inner/blue.txt"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        inner.open("../inner/red.txt"),
        "a path led outside of the filesystem"
    );

    #[cfg(not(windows))]
    error!(inner.open_dir(""), "No such file");
    #[cfg(windows)]
    error!(inner.open_dir(""), 2);

    error_contains!(inner.open_dir("/"), "a path led outside of the filesystem");
    error_contains!(
        inner.open_dir("/etc/services"),
        "a path led outside of the filesystem"
    );
    check!(inner.open_dir("."));
    check!(inner.open_dir("./"));
    check!(inner.open_dir("./."));
    error_contains!(inner.open_dir(".."), "a path led outside of the filesystem");
    error_contains!(
        inner.open_dir("../"),
        "a path led outside of the filesystem"
    );
    error_contains!(
        inner.open_dir("../."),
        "a path led outside of the filesystem"
    );
    error_contains!(
        inner.open_dir("./.."),
        "a path led outside of the filesystem"
    );
}

#[test]
fn symlinks() {
    #[cfg(windows)]
    use cap_fs_ext::DirExt;

    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));
    check!(tmpdir.write("red.txt", b"hello world\n"));
    check!(tmpdir.write("dir/green.txt", b"goodmight moon\n"));
    check!(tmpdir.write("dir/inner/blue.txt", b"hey mars\n"));

    let inner = check!(tmpdir.open_dir("dir/inner"));

    check!(tmpdir.symlink("dir", "link"));
    #[cfg(not(windows))]
    check!(tmpdir.symlink("does_not_exist", "badlink"));

    check!(tmpdir.open("link/../red.txt"));
    check!(tmpdir.open("link/green.txt"));
    check!(tmpdir.open("link/inner/blue.txt"));
    #[cfg(not(windows))]
    {
        error_contains!(tmpdir.open("link/red.txt"), "No such file");
        error_contains!(tmpdir.open("link/../green.txt"), "No such file");
    }
    #[cfg(windows)]
    {
        error_contains!(
            tmpdir.open("link/red.txt"),
            "The system cannot find the file specified."
        );
        error_contains!(
            tmpdir.open("link/../green.txt"),
            "The system cannot find the file specified."
        );
    }

    check!(tmpdir.open("./dir/.././/link/..///./red.txt"));
    check!(tmpdir.open("link/inner/../inner/../../red.txt"));
    error_contains!(
        inner.open("../inner/../inner/../../link/other.txt"),
        "a path led outside of the filesystem"
    );
    #[cfg(not(windows))]
    {
        error_contains!(
            tmpdir.open("./dir/.././/link/..///./not.txt"),
            "No such file"
        );
        error_contains!(tmpdir.open("link/other.txt"), "No such file");
        error_contains!(tmpdir.open("badlink/../red.txt"), "No such file");
    }
    #[cfg(windows)]
    {
        error_contains!(
            tmpdir.open("./dir/.././/link/..///./not.txt"),
            "The system cannot find the file specified."
        );
        error_contains!(
            tmpdir.open("link/other.txt"),
            "The system cannot find the file specified."
        );
    }
}

#[test]
#[cfg(not(windows))]
fn symlink_loop() {
    #[cfg(windows)]
    use cap_fs_ext::DirExt;

    let tmpdir = tmpdir();
    check!(tmpdir.symlink("link", "link"));
    // TODO: Check the error message
    error_contains!(tmpdir.open("link"), "");
}

#[test]
fn symlink_loop_from_rename() {
    #[cfg(windows)]
    use cap_fs_ext::DirExt;

    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();
    check!(tmpdir.create("file"));
    check!(tmpdir.symlink("file", "link"));
    check!(tmpdir.open("link"));
    check!(tmpdir.rename("file", &tmpdir, "renamed"));
    error_contains!(tmpdir.open("link"), "");
    check!(tmpdir.rename("link", &tmpdir, "file"));
    error_contains!(tmpdir.open("file"), "");
    check!(tmpdir.rename("file", &tmpdir, "link"));
    error_contains!(tmpdir.open("link"), "");
    check!(tmpdir.rename("renamed", &tmpdir, "file"));
    check!(tmpdir.open("link"));
}

#[cfg(target_os = "linux")]
#[test]
fn proc_self_fd() {
    let fd = check!(std::fs::File::open("/proc/self/fd"));
    let dir = cap_std::fs::Dir::from_std_file(fd);
    // This should fail with "too many levels of symbolic links".
    dir.open("0").unwrap_err();
}
