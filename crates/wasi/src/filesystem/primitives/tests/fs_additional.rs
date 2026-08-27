// This file contains additional fs tests that didn't make it into `fs.rs`.
// The reason for additional module to contain those is so that `fs.rs` mirrors
// Rust's libstd tests.
use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_supported;
use crate::filesystem::primitives as p;
use std::io::{Read, Write};
use std::path::Path;
use std::str;

#[test]
fn dir_writable() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "dir"));
    #[cfg(not(windows))]
    error_contains!(h::create(&start, "dir"), "Is a directory");
    #[cfg(windows)]
    error!(h::create(&start, "dir"), 5);
    error_contains!(
        p::open(&start, Path::new("dir"), p::OpenOptions::new().write(true)),
        "Is a directory"
    );

    error_contains!(h::create(&start, "dir/."), "Is a directory");
    error_contains!(
        p::open(
            &start,
            Path::new("dir/."),
            p::OpenOptions::new().write(true)
        ),
        "Is a directory"
    );

    error_contains!(h::create(&start, "dir/.."), "Is a directory");
    error_contains!(
        p::open(
            &start,
            Path::new("dir/.."),
            p::OpenOptions::new().write(true)
        ),
        "Is a directory"
    );
}

#[test]
fn readdir_write() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "dir"));
    assert!(p::open(&start, Path::new("dir"), p::OpenOptions::new().write(true)).is_err());
    assert!(p::open(&start, Path::new("dir/"), p::OpenOptions::new().write(true)).is_err());

    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        use crate::filesystem::primitives::OpenOptionsExt;
        assert!(
            p::open(
                &start,
                Path::new("dir"),
                p::OpenOptions::new()
                    .write(true)
                    .custom_flags(rustix::fs::OFlags::DIRECTORY.bits() as i32)
            )
            .is_err()
        );
    }
}

#[test]
fn maybe_dir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "dir"));

    // Opening directories works on non-Windows platforms.
    #[cfg(not(windows))]
    check!(h::open(&start, "dir"));

    // Opening directories fails on Windows.
    #[cfg(windows)]
    assert!(h::open(&start, "dir").is_err());
}

#[test]
fn optionally_recursive_mkdir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "d1/d2";
    check!(h::create_dir_all(&start, dir));
    assert!(h::is_dir(&start, dir));
}

#[test]
fn optionally_nonrecursive_mkdir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "d1/d2";
    #[cfg(not(windows))]
    error!(
        p::create_dir(&start, Path::new(dir), &p::DirOptions::new()),
        "No such file"
    );
    #[cfg(windows)]
    error!(
        p::create_dir(&start, Path::new(dir), &p::DirOptions::new()),
        2
    );

    assert!(!h::exists(&start, dir));
}

#[test]
fn dotdot_at_end_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir(&start, "b"));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "..", "up"));

    // Do some things with `path` that might break with an `O_PATH` fd.
    // The `permissions` part of this test is gone with `set_permissions`, but
    // the `read_dir` part is the part that exercises the `O_PATH` fd.
    let path = "b/up";

    check!(h::metadata(&start, path));

    let contents = check!(h::read_dir(&start, path));
    for entry in contents {
        let _entry = check!(entry);
    }
}

#[test]
fn dotdot_at_end_of_symlink_all_inside_dir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::create_dir(&start, "dir"));
    check!(h::write(&start, "dir/target", foo));
    check!(h::create_dir(&start, "dir/b"));
    let b = check!(p::open_dir(&start, Path::new("dir/b")));
    check!(h::symlink_dir(&b, "..", "up"));

    // Do some things with `path` that might break with an `O_PATH` fd.
    // The `permissions` part of this test is gone with `set_permissions`, but
    // the `read_dir` part is the part that exercises the `O_PATH` fd.
    let path = "dir/b/up";

    check!(h::metadata(&start, path));

    let contents = check!(h::read_dir(&start, path));
    for entry in contents {
        let _entry = check!(entry);
    }
}

#[test]
fn dotdot_slashdot_at_end_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir(&start, "b"));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "../.", "up"));

    // Do some things with `path` that might break with an `O_PATH` fd.
    // The `permissions` part of this test is gone with `set_permissions`, but
    // the `read_dir` part is the part that exercises the `O_PATH` fd.
    let path = "b/up";

    check!(h::metadata(&start, path));

    let contents = check!(h::read_dir(&start, path));
    for entry in contents {
        let _entry = check!(entry);
    }
}

#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_slashdot_at_end_of_symlink_all_inside_dir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::create_dir(&start, "dir"));
    check!(h::write(&start, "dir/target", foo));
    check!(h::create_dir(&start, "dir/b"));
    let b = check!(p::open_dir(&start, Path::new("dir/b")));
    check!(h::symlink_dir(&b, "../.", "up"));

    // Do some things with `path` that might break with an `O_PATH` fd.
    // The `permissions` part of this test is gone with `set_permissions`, but
    // the `read_dir` part is the part that exercises the `O_PATH` fd.
    let path = "dir/b/up";

    check!(h::metadata(&start, path));

    let contents = check!(h::read_dir(&start, path));
    for entry in contents {
        let _entry = check!(entry);
    }
}

#[test]
fn recursive_mkdir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "d1/d2";
    check!(h::create_dir_all(&start, dir));
    assert!(h::is_dir(&start, "d1"));
    let dir = check!(p::open_dir(&start, Path::new("d1")));
    assert!(h::is_dir(&dir, "d2"));
    assert!(h::is_dir(&start, "d1/d2"));
}

#[test]
#[cfg_attr(windows, ignore)] // TODO investigate why this one is failing
fn open_various() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    #[cfg(not(windows))]
    error!(h::create(&start, ""), "No such file");
    #[cfg(windows)]
    error!(h::create(&start, ""), 2);

    #[cfg(not(windows))]
    error!(h::create(&start, "."), "Is a directory");
    #[cfg(windows)]
    error!(h::create(&start, "."), 2);
}

#[test]
fn trailing_slash() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create(&start, "file"));

    #[cfg(not(windows))]
    {
        error!(h::open(&start, "file/../file"), "Not a directory");
        error!(h::open(&start, "file/.."), "Not a directory");
        error!(h::open(&start, "file/."), "Not a directory");
        error!(h::open(&start, "file/../file/"), "Not a directory");
        error!(h::open(&start, "file/"), "Not a directory");
        error!(
            p::open_dir(&start, Path::new("file/../file/")),
            "Not a directory"
        );
        error!(
            p::open_dir(&start, Path::new("file/../file")),
            "Not a directory"
        );
        error!(p::open_dir(&start, Path::new("file/..")), "Not a directory");
        error!(p::open_dir(&start, Path::new("file/.")), "Not a directory");
        error!(p::open_dir(&start, Path::new("file/")), "Not a directory");
    }

    #[cfg(windows)]
    {
        assert!(check!(check!(h::open(&start, "file/../file")).metadata()).is_file());
        assert!(
            check!(p::Metadata::from_file(&check!(p::open_dir(
                &start,
                Path::new("file/..")
            ))))
            .is_dir()
        );
        assert!(check!(check!(h::open(&start, "file/.")).metadata()).is_file());
        assert!(p::open_dir(&start, Path::new("file/../file/")).is_err());
        assert!(p::open_dir(&start, Path::new("file/./")).is_err());
        assert!(p::open_dir(&start, Path::new("file//")).is_err());
        assert!(p::open_dir(&start, Path::new("file/../file")).is_err());
        assert!(p::open_dir(&start, Path::new("file/.")).is_err());
        assert!(p::open_dir(&start, Path::new("file/")).is_err());
    }
}

#[test]
fn trailing_slash_in_dir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "dir"));
    check!(h::create(&start, "dir/file"));

    #[cfg(not(windows))]
    {
        error!(h::open(&start, "dir/file/../file"), "Not a directory");
        error!(h::open(&start, "dir/file/.."), "Not a directory");
        error!(h::open(&start, "dir/file/."), "Not a directory");
        error!(h::open(&start, "dir/file/../file/"), "Not a directory");
        error!(h::open(&start, "dir/file/"), "Not a directory");
        error!(
            p::open_dir(&start, Path::new("dir/file/../file/")),
            "Not a directory"
        );
        error!(
            p::open_dir(&start, Path::new("dir/file/../file")),
            "Not a directory"
        );
        error!(
            p::open_dir(&start, Path::new("dir/file/..")),
            "Not a directory"
        );
        error!(
            p::open_dir(&start, Path::new("dir/file/.")),
            "Not a directory"
        );
        error!(
            p::open_dir(&start, Path::new("dir/file/")),
            "Not a directory"
        );
    }

    #[cfg(windows)]
    {
        assert!(check!(check!(h::open(&start, "dir/file/../file")).metadata()).is_file());
        assert!(
            check!(p::Metadata::from_file(&check!(p::open_dir(
                &start,
                Path::new("dir/file/..")
            ))))
            .is_dir()
        );
        assert!(check!(check!(h::open(&start, "dir/file/.")).metadata()).is_file());
        assert!(h::open(&start, "dir/file/../file/").is_err());
        let _ = check!(h::open(&start, "dir/file/../file/."));
        assert!(h::open(&start, "dir/file/../file/./").is_err());
        assert!(h::open(&start, "dir/file/").is_err());
        let _ = check!(h::open(&start, "dir/file/."));
        let _ = check!(h::open(&start, "dir/file/../file/."));
        assert!(h::open(&start, "dir/file/../file/./").is_err());
        assert!(h::open(&start, "dir/file/").is_err());
        let _ = check!(h::open(&start, "dir/file/."));
        assert!(h::open(&start, "dir/file/./").is_err());
        assert!(p::open_dir(&start, Path::new("dir/file/../file/")).is_err());
        assert!(p::open_dir(&start, Path::new("dir/file/../file/.")).is_err());
        assert!(p::open_dir(&start, Path::new("dir/file/../file/./")).is_err());
        assert!(p::open_dir(&start, Path::new("dir/file/../file")).is_err());
        assert!(p::open_dir(&start, Path::new("dir/file/.")).is_err());
        assert!(p::open_dir(&start, Path::new("dir/file/./")).is_err());
        assert!(p::open_dir(&start, Path::new("dir/file/")).is_err());
    }
}

#[test]
#[cfg_attr(windows, ignore)] // TODO investigate why this one is failing
fn rename_slashdots() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "dir"));
    check!(p::rename(
        &start,
        Path::new("dir"),
        &start,
        Path::new("dir")
    ));
    check!(p::rename(
        &start,
        Path::new("dir"),
        &start,
        Path::new("dir/")
    ));
    check!(p::rename(
        &start,
        Path::new("dir/"),
        &start,
        Path::new("dir")
    ));
    check!(p::rename(
        &start,
        Path::new("dir/"),
        &start,
        Path::new("dir/")
    ));

    // TODO: Platform-specific error code.
    error_contains!(
        p::rename(&start, Path::new("dir"), &start, Path::new("dir/.")),
        ""
    );
    error_contains!(
        p::rename(&start, Path::new("dir/."), &start, Path::new("dir")),
        ""
    );
}

#[test]
#[cfg_attr(windows, ignore)] // TODO investigate why this one is failing
fn rename_slashdots_ambient() {
    let dir = tempfile::tempdir().unwrap();

    check!(std::fs::create_dir_all(dir.path().join("dir")));
    check!(std::fs::rename(
        dir.path().join("dir"),
        dir.path().join("dir")
    ));
    check!(std::fs::rename(
        dir.path().join("dir"),
        dir.path().join("dir/")
    ));
    check!(std::fs::rename(
        dir.path().join("dir/"),
        dir.path().join("dir")
    ));
    check!(std::fs::rename(
        dir.path().join("dir/"),
        dir.path().join("dir/")
    ));

    // TODO: Platform-specific error code.
    error_contains!(
        std::fs::rename(dir.path().join("dir"), dir.path().join("dir/.")),
        ""
    );
    error_contains!(
        std::fs::rename(dir.path().join("dir/."), dir.path().join("dir")),
        ""
    );
}

#[test]
fn try_exists() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    assert_eq!(h::exists(&start, "somefile"), false);
    let dir = Path::new("d1/d2");
    let parent = dir.parent().unwrap();
    assert_eq!(h::exists(&start, parent), false);
    assert_eq!(h::exists(&start, dir), false);
    check!(h::create_dir(&start, parent));
    assert_eq!(h::exists(&start, parent), true);
    assert_eq!(h::exists(&start, dir), false);
    check!(h::create_dir(&start, dir));
    assert_eq!(h::exists(&start, dir), true);
}

#[test]
fn file_test_directoryinfo_readdir() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    let dir = "di_readdir";
    check!(h::create_dir(&start, dir));
    let prefix = "foo";
    for n in 0..3 {
        let f = format!("{n}.txt");
        let mut w = check!(h::create(&start, &f));
        let msg_str = format!("{}{}", prefix, n.to_string());
        let msg = msg_str.as_bytes();
        check!(w.write(msg));
    }
    let sub = check!(p::open_dir(&start, Path::new(dir)));
    let files = check!(p::read_base_dir(&sub));
    let mut mem = [0; 4];
    for f in files {
        let f = f.unwrap();
        {
            check!(check!(h::open(&sub, f.file_name())).read(&mut mem));
            let read_str = str::from_utf8(&mem).unwrap();
            let expected = format!("{}{}", prefix, f.file_name().to_str().unwrap());
            assert_eq!(expected, read_str);
        }
        check!(p::remove_file(&sub, Path::new(&f.file_name())));
    }
    drop(sub);
    check!(p::remove_dir(&start, Path::new(dir)));
}

#[test]
fn follow_dotdot_symlink() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&start, "a/b"));
    check!(h::symlink_dir(&start, "..", "a/b/c"));
    check!(h::symlink_dir(&start, "../..", "a/b/d"));
    check!(h::symlink_dir(&start, "../../..", "a/b/e"));
    check!(h::symlink_dir(&start, "../../../..", "a/b/f"));

    check!(p::open_dir(&start, Path::new("a/b/c")));
    assert!(check!(h::metadata(&start, "a/b/c")).is_dir());

    #[cfg(windows)]
    {
        error!(p::open_dir(&start, Path::new("a/b/d")), 123);
        error!(h::metadata(&start, "a/b/d"), 123);

        error!(p::open_dir(&start, Path::new("a/b/e")), 123);
        error!(h::metadata(&start, "a/b/e"), 123);

        error!(p::open_dir(&start, Path::new("a/b/f")), 123);
        error!(h::metadata(&start, "a/b/f"), 123);
    }

    #[cfg(not(windows))]
    {
        check!(p::open_dir(&start, Path::new("a/b/d")));
        assert!(check!(h::metadata(&start, "a/b/d")).is_dir());

        assert!(p::open_dir(&start, Path::new("a/b/e")).is_err());
        assert!(h::metadata(&start, "a/b/e").is_err());

        assert!(p::open_dir(&start, Path::new("a/b/f")).is_err());
        assert!(h::metadata(&start, "a/b/f").is_err());
    }
}

#[test]
fn follow_dotdot_symlink_ambient() {
    #[cfg(unix)]
    use std::os::unix::fs::symlink as symlink_dir;
    #[cfg(windows)]
    use std::os::windows::fs::symlink_dir;

    if !symlink_supported() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    check!(std::fs::create_dir_all(dir.path().join("a/b")));
    check!(symlink_dir("..", dir.path().join("a/b/c")));
    check!(symlink_dir("../..", dir.path().join("a/b/d")));
    check!(symlink_dir("../../..", dir.path().join("a/b/e")));
    check!(symlink_dir("../../../..", dir.path().join("a/b/f")));

    check!(h::open_ambient_dir(dir.path().join("a/b/c")));
    assert!(check!(std::fs::metadata(dir.path().join("a/b/c"))).is_dir());

    #[cfg(windows)]
    {
        error!(h::open_ambient_dir(dir.path().join("a/b/d")), 123);
        error!(std::fs::metadata(dir.path().join("a/b/d")), 123);

        error!(h::open_ambient_dir(dir.path().join("a/b/e")), 123);
        error!(std::fs::metadata(dir.path().join("a/b/e")), 123);

        error!(h::open_ambient_dir(dir.path().join("a/b/f")), 123);
        error!(std::fs::metadata(dir.path().join("a/b/f")), 123);
    }

    #[cfg(not(windows))]
    {
        check!(h::open_ambient_dir(dir.path().join("a/b/d")));
        assert!(check!(std::fs::metadata(dir.path().join("a/b/d"))).is_dir());

        check!(h::open_ambient_dir(dir.path().join("a/b/e")));
        assert!(check!(std::fs::metadata(dir.path().join("a/b/e"))).is_dir());

        check!(h::open_ambient_dir(dir.path().join("a/b/f")));
        assert!(check!(std::fs::metadata(dir.path().join("a/b/f"))).is_dir());
    }
}

#[test]
fn follow_file_symlink() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);

    check!(h::create(&start, "file"));

    check!(h::symlink_file(&start, "file", "link"));
    check!(h::symlink_dir(&start, "file/", "link_slash"));
    check!(h::symlink_file(&start, "file/.", "link_slashdot"));
    check!(h::symlink_dir(&start, "file/..", "link_slashdotdot"));

    check!(h::open(&start, "link"));
    assert!(h::open(&start, "link_slash").is_err());

    #[cfg(windows)]
    {
        error!(h::open(&start, "link_slashdot"), 123);
        error!(p::open_dir(&start, Path::new("link_slashdotdot")), 123);
    }
    #[cfg(not(windows))]
    {
        assert!(h::open(&start, "link_slash").is_err());
        assert!(h::open(&start, "link_slashdot").is_err());
        assert!(p::open_dir(&start, Path::new("link_slashdotdot")).is_err());
    }
}

/// This test is the same as `check_dot_access` but uses `std::fs`'
/// ambient API instead of `cap_std`. The purpose of this test is to
/// confirm fundamentally OS-specific differences.
#[cfg(unix)]
#[test]
fn check_dot_access_ambient() {
    use std::fs;
    use std::os::unix::fs::DirBuilderExt;

    let dir = tempfile::tempdir().unwrap();

    let mut options = std::fs::DirBuilder::new();
    options.mode(0o477);
    check!(options.create(dir.path().join("dir")));

    check!(fs::metadata(dir.path().join(".")));
    check!(fs::metadata(dir.path().join("dir")));
    check!(fs::metadata(dir.path().join("dir/")));
    check!(fs::metadata(dir.path().join("dir//")));

    if !cfg!(target_os = "freebsd") {
        assert!(fs::metadata(dir.path().join("dir/.")).is_err());
        assert!(fs::metadata(dir.path().join("dir/./")).is_err());
        assert!(fs::metadata(dir.path().join("dir/.//")).is_err());
        assert!(fs::metadata(dir.path().join("dir/./.")).is_err());
        assert!(fs::metadata(dir.path().join("dir/.//.")).is_err());
        assert!(fs::metadata(dir.path().join("dir/..")).is_err());
        assert!(fs::metadata(dir.path().join("dir/../")).is_err());
        assert!(fs::metadata(dir.path().join("dir/..//")).is_err());
        assert!(fs::metadata(dir.path().join("dir/../.")).is_err());
        assert!(fs::metadata(dir.path().join("dir/..//.")).is_err());
    }
}

// Windows allows one to open "file/." and "file/.." and similar, however it
// doesn't allow "file/" or similar.
#[cfg(windows)]
#[test]
fn file_with_trailing_slashdot() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create(&start, "file"));
    check!(h::open(&start, "file"));
    check!(h::open(&start, "file\\."));
    check!(h::open(&start, "file/."));
    check!(h::open(&start, "file\\.\\."));
    check!(h::open(&start, "file/./."));
    assert!(h::open(&start, "file\\").is_err());
    assert!(h::open(&start, "file/").is_err());
    assert!(h::open(&start, "file\\.\\").is_err());
    assert!(h::open(&start, "file/./").is_err());
    check!(p::open_dir(&start, Path::new("file\\..")));
    check!(p::open_dir(&start, Path::new("file/..")));
    check!(p::open_dir(&start, Path::new("file\\.\\..")));
    check!(p::open_dir(&start, Path::new("file/./..")));
    check!(p::open_dir(&start, Path::new("file\\..\\.")));
    check!(p::open_dir(&start, Path::new("file/../.")));
    check!(p::open_dir(&start, Path::new("file\\..\\")));
    check!(p::open_dir(&start, Path::new("file/../")));
    assert!(p::open_dir(&start, Path::new("file\\...")).is_err());
    assert!(p::open_dir(&start, Path::new("file/...")).is_err());
}

/// This is just to confirm that Windows really does allow one to open "file/."
/// and "file/..", and similar, however it doesn't allow "file/" or similar.
#[cfg(windows)]
#[test]
fn file_with_trailing_slashdot_ambient() {
    let dir = tempfile::tempdir().unwrap();
    check!(std::fs::File::create(dir.path().join("file")));
    check!(std::fs::File::open(dir.path().join("file")));
    check!(std::fs::File::open(dir.path().join("file\\.")));
    check!(std::fs::File::open(dir.path().join("file/.")));
    check!(std::fs::File::open(dir.path().join("file\\.\\.")));
    check!(std::fs::File::open(dir.path().join("file/./.")));
    assert!(std::fs::File::open(dir.path().join("file\\")).is_err());
    assert!(std::fs::File::open(dir.path().join("file/")).is_err());
    assert!(std::fs::File::open(dir.path().join("file\\.\\")).is_err());
    assert!(std::fs::File::open(dir.path().join("file/./")).is_err());
    check!(h::open_ambient_dir(dir.path().join("file/..")));
    check!(h::open_ambient_dir(dir.path().join("file\\.\\..")));
    check!(h::open_ambient_dir(dir.path().join("file/./..")));
    check!(h::open_ambient_dir(dir.path().join("file\\..\\.")));
    check!(h::open_ambient_dir(dir.path().join("file/../.")));
    check!(h::open_ambient_dir(dir.path().join("file\\..\\")));
    check!(h::open_ambient_dir(dir.path().join("file/../")));
    assert!(h::open_ambient_dir(dir.path().join("file\\...")).is_err());
    assert!(h::open_ambient_dir(dir.path().join("file/...")).is_err());
}

#[cfg(all(
    unix,
    not(any(
        target_os = "ios",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "visionos",
    ))
))]

/// This test is the same as `dir_searchable_unreadable` but uses `std::fs`'
/// ambient API instead of `cap_std`. The purpose of this test is to
/// confirm fundamentally OS-specific differences.
#[cfg(all(
    unix,
    not(any(
        target_os = "ios",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "visionos",
    ))
))]
#[test]
fn dir_searchable_unreadable_ambient() {
    use std::fs;
    use std::os::unix::fs::DirBuilderExt;

    let dir = tempfile::tempdir().unwrap();

    let mut options = std::fs::DirBuilder::new();
    options.mode(0o333);
    check!(options.create(dir.path().join("dir")));
    check!(options.create(dir.path().join("dir/writeable_subdir")));
    options.mode(0o111);
    check!(options.create(dir.path().join("dir/subdir")));

    assert!(check!(fs::metadata(dir.path().join("dir/."))).is_dir());
    assert!(check!(fs::metadata(dir.path().join("dir/subdir"))).is_dir());
    assert!(check!(fs::metadata(dir.path().join("dir/subdir/."))).is_dir());
}

/// On Darwin, we don't have a race-free way to create a subdirectory within
/// a directory that we don't have read access to.
#[cfg(any(
    target_os = "ios",
    target_os = "macos",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "visionos",
))]

/// Like `dir_unsearchable_unreadable`, but uses ambient-authority APIs
/// to test underlying host functionality.
#[cfg(unix)]
#[test]
fn dir_unsearchable_unreadable_ambient() {
    use std::fs::DirBuilder;
    use std::os::unix::fs::DirBuilderExt;

    let dir = tempfile::tempdir().unwrap();

    let mut options = DirBuilder::new();
    options.mode(0o000);
    check!(options.create(dir.path().join("dir")));

    if cfg!(any(
        target_os = "android",
        target_os = "linux",
        target_os = "redox",
    )) {
        assert!(std::fs::File::open(dir.path().join("dir")).is_err());
        assert!(std::fs::read_dir(dir.path().join("dir")).is_err());
        assert!(std::fs::File::open(dir.path().join("dir/.")).is_err());
    }
}

/// This test is the same as `symlink_hard_link` but uses `std::fs`'
/// ambient API instead of `cap_std`. The purpose of this test is to
/// confirm fundamentally OS-specific behaviors.
#[test]
fn symlink_hard_link_ambient() {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    #[cfg(windows)]
    use std::os::windows::fs::symlink_file;

    if !symlink_supported() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();

    check!(std::fs::File::create(dir.path().join("file")));
    #[cfg(not(windows))]
    check!(symlink("file", dir.path().join("symlink")));
    #[cfg(windows)]
    check!(symlink_file("file", dir.path().join("symlink")));
    check!(std::fs::hard_link(
        dir.path().join("symlink"),
        dir.path().join("hard_link")
    ));
    assert!(
        check!(std::fs::symlink_metadata(dir.path().join("hard_link")))
            .file_type()
            .is_symlink()
    );
    let _ = check!(std::fs::File::open(dir.path().join("file")));
    assert!(std::fs::File::open(dir.path().join("file.renamed")).is_err());
    let _ = check!(std::fs::File::open(dir.path().join("symlink")));
    let _ = check!(std::fs::File::open(dir.path().join("hard_link")));
    check!(std::fs::rename(
        dir.path().join("file"),
        dir.path().join("file.renamed")
    ));
    assert!(std::fs::File::open(dir.path().join("file")).is_err());
    let _ = check!(std::fs::File::open(dir.path().join("file.renamed")));
    assert!(std::fs::File::open(dir.path().join("symlink")).is_err());
    assert!(std::fs::File::open(dir.path().join("hard_link")).is_err());
    assert!(std::fs::read_link(dir.path().join("file")).is_err());
    assert!(std::fs::read_link(dir.path().join("file.renamed")).is_err());
    assert_eq!(
        check!(std::fs::read_link(dir.path().join("symlink"))),
        Path::new("file")
    );
    assert_eq!(
        check!(std::fs::read_link(dir.path().join("hard_link"))),
        Path::new("file")
    );
    check!(std::fs::remove_file(dir.path().join("file.renamed")));
    assert!(std::fs::File::open(dir.path().join("file")).is_err());
    assert!(std::fs::File::open(dir.path().join("file.renamed")).is_err());
    assert!(std::fs::File::open(dir.path().join("symlink")).is_err());
    assert!(std::fs::File::open(dir.path().join("hard_link")).is_err());
    assert!(
        check!(std::fs::symlink_metadata(dir.path().join("hard_link")))
            .file_type()
            .is_symlink()
    );
}

/// POSIX says that whether or not `link` follows symlinks in the `old`
/// path is implementation-defined. We want `hard_link` to not follow
/// symbolic links.
#[test]
fn symlink_hard_link() {
    if !symlink_supported() {
        return;
    }

    let tmpdir = tmpdir();

    let start = h::dir_of(&tmpdir);

    check!(h::create(&start, "file"));
    check!(h::symlink_file(&start, "file", "symlink"));
    check!(p::hard_link(
        &start,
        Path::new("symlink"),
        &start,
        Path::new("hard_link")
    ));
    assert!(
        check!(h::symlink_metadata(&start, "hard_link"))
            .file_type()
            .is_symlink()
    );
    let _ = check!(h::open(&start, "file"));
    assert!(h::open(&start, "file.renamed").is_err());
    let _ = check!(h::open(&start, "symlink"));
    let _ = check!(h::open(&start, "hard_link"));
    check!(p::rename(
        &start,
        Path::new("file"),
        &start,
        Path::new("file.renamed")
    ));
    assert!(h::open(&start, "file").is_err());
    let _ = check!(h::open(&start, "file.renamed"));
    assert!(h::open(&start, "symlink").is_err());
    assert!(h::open(&start, "hard_link").is_err());
    assert!(p::read_link(&start, Path::new("file")).is_err());
    assert!(p::read_link(&start, Path::new("file.renamed")).is_err());
    assert_eq!(
        check!(p::read_link(&start, Path::new("symlink"))),
        Path::new("file")
    );
    assert_eq!(
        check!(p::read_link(&start, Path::new("hard_link"))),
        Path::new("file")
    );
    check!(p::remove_file(&start, Path::new("file.renamed")));
    assert!(h::open(&start, "file").is_err());
    assert!(h::open(&start, "file.renamed").is_err());
    assert!(h::open(&start, "symlink").is_err());
    assert!(h::open(&start, "hard_link").is_err());
    assert!(
        check!(h::symlink_metadata(&start, "hard_link"))
            .file_type()
            .is_symlink()
    );
}

#[test]
fn readdir_with_trailing_slashdot() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "dir"));
    check!(h::create(&start, "dir/red"));
    check!(h::create(&start, "dir/green"));
    check!(h::create(&start, "dir/blue"));

    assert_eq!(check!(h::read_dir(&start, "dir")).count(), 3);
    assert_eq!(check!(h::read_dir(&start, "dir/")).count(), 3);
    assert_eq!(check!(h::read_dir(&start, "dir/.")).count(), 3);
}

#[test]
fn metadata_vs_std_fs() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create_dir(&start, "dir"));
    let dir = check!(p::open_dir(&start, Path::new("dir")));
    let file = check!(h::create(&dir, "file"));

    let cap_std_dir = check!(p::Metadata::from_file(&dir));
    let cap_std_file = check!(p::Metadata::from_file(&file));
    let cap_std_dir_entry = {
        let mut entries = check!(p::read_base_dir(&dir));
        let entry = check!(entries.next().unwrap());
        assert_eq!(entry.file_name(), "file");
        assert!(entries.next().is_none(), "unexpected dir entry");
        check!(entry.metadata())
    };

    let std_dir = check!(dir.metadata());
    let std_file = check!(file.metadata());

    match std_dir.created() {
        Ok(_) => println!("std::fs supports file created times"),
        Err(e) => println!("std::fs doesn't support file created times: {e}"),
    }

    check_metadata(&std_dir, &cap_std_dir);
    check_metadata(&std_file, &cap_std_file);
    check_metadata(&std_file, &cap_std_dir_entry);
}

fn check_metadata(std: &std::fs::Metadata, cap: &p::Metadata) {
    assert_eq!(std.is_dir(), cap.is_dir());
    assert_eq!(std.is_file(), cap.file_type().is_file());
    assert_eq!(std.is_symlink(), cap.file_type().is_symlink());
    assert_eq!(std.file_type().is_dir(), cap.file_type().is_dir());
    assert_eq!(std.file_type().is_file(), cap.file_type().is_file());
    assert_eq!(std.file_type().is_symlink(), cap.file_type().is_symlink());
    #[cfg(unix)]
    {
        assert_eq!(
            std::os::unix::fs::FileTypeExt::is_block_device(&std.file_type()),
            p::FileTypeExt::is_block_device(&cap.file_type())
        );
        assert_eq!(
            std::os::unix::fs::FileTypeExt::is_char_device(&std.file_type()),
            p::FileTypeExt::is_char_device(&cap.file_type())
        );
    }

    assert_eq!(std.len(), cap.len());

    // If the standard library supports file modified/accessed/created times,
    // then the primitives should too.
    match std.modified() {
        Ok(expected) => assert_eq!(expected, check!(cap.modified())),
        Err(e) => assert!(
            cap.modified().is_err(),
            "modified time should be error ({}), got {:#?}",
            e,
            cap.modified()
        ),
    }
    // The access times might be a little different due to either our own
    // or concurrent accesses.
    const ACCESS_TOLERANCE_SEC: u32 = 60;
    match std.accessed() {
        Ok(expected) => {
            let access_tolerance = std::time::Duration::from_secs(ACCESS_TOLERANCE_SEC.into());
            assert!(
                ((expected - access_tolerance)..(expected + access_tolerance))
                    .contains(&check!(cap.accessed())),
                "std accessed {:#?}, cap accessed {:#?}",
                expected,
                cap.accessed()
            );
        }
        Err(e) => assert!(
            cap.accessed().is_err(),
            "accessed time should be error ({}), got {:#?}",
            e,
            cap.accessed()
        ),
    }
    match std.created() {
        Ok(expected) => assert_eq!(expected, check!(cap.created())),
        Err(e) => {
            // An earlier bug returned the Unix epoch instead of `None` when
            // created times were unavailable. This tries to catch such errors,
            // while also allowing some targets to return valid created times
            // even when std doesn't.
            if let Ok(actual) = cap.created() {
                println!("std returned error for created time ({e}) but got {actual:#?}");
                assert_ne!(actual, std::time::SystemTime::UNIX_EPOCH);
            }
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(std.dev(), p::MetadataExt::dev(cap));
        assert_eq!(std.ino(), p::MetadataExt::ino(cap));
        assert_eq!(std.nlink(), p::MetadataExt::nlink(cap));
    }
}

/// Test that a symlink in the middle of a path containing ".." doesn't cause
/// the path to be treated as if it ends with "..".
#[test]
fn dotdot_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir(&start, "b"));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "..", "up"));

    let path = "b/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Like `dotdot_in_middle_of_symlink` but with a `/.` at the end.
///
/// Windows doesn't appear to like symlinks that end with `/.`.
#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_slashdot_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir(&start, "b"));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "../.", "up"));

    let path = "b/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Same as `dotdot_in_middle_of_symlink`, but use two levels of `..`.
///
/// Windows doesn't appear to like symlinks that end with `/..`.
#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_more_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir_all(&start, "b/c"));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "c/../..", "up"));

    let path = "b/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Like `dotdot_more_in_middle_of_symlink`, but with a `/.` at the end.
///
/// Windows doesn't appear to like symlinks that end with `/.`.
#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_slashdot_more_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir_all(&start, "b/c"));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "c/../../.", "up"));

    let path = "b/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Same as `dotdot_more_in_middle_of_symlink`, but the symlink doesn't
/// include `c`.
///
/// Windows doesn't appear to like symlinks that end with `/..`.
#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_other_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir_all(&start, "b/c"));
    let c = check!(p::open_dir(&start, Path::new("b/c")));
    check!(h::symlink_dir(&c, "../..", "up"));

    let path = "b/c/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Like `dotdot_other_in_middle_of_symlink`, but with `/.` at the end.
///
/// Windows doesn't appear to like symlinks that end with `/.`.
#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_slashdot_other_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::write(&start, "target", foo));
    check!(h::create_dir_all(&start, "b/c"));
    let c = check!(p::open_dir(&start, Path::new("b/c")));
    check!(h::symlink_dir(&c, "../../.", "up"));

    let path = "b/c/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Same as `dotdot_more_in_middle_of_symlink`, but use a symlink that
/// doesn't end with `..`.
#[test]
fn dotdot_even_more_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::create_dir_all(&start, "b/c"));
    check!(h::write(&start, "b/target", foo));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "c/../../b", "up"));

    let path = "b/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Like `dotdot_even_more_in_middle_of_symlink`, but with a `/.` at the end.
///
/// Windows doesn't appear to like symlinks that end with `/.`.
#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_slashdot_even_more_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::create_dir_all(&start, "b/c"));
    check!(h::write(&start, "b/target", foo));
    let b = check!(p::open_dir(&start, Path::new("b")));
    check!(h::symlink_dir(&b, "c/../../b/.", "up"));

    let path = "b/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Same as `dotdot_even_more_in_middle_of_symlink`, but the symlink doesn't
/// include `c`.
#[test]
fn dotdot_even_other_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::create_dir_all(&start, "b/c"));
    check!(h::write(&start, "b/target", foo));
    let c = check!(p::open_dir(&start, Path::new("b/c")));
    check!(h::symlink_dir(&c, "../../b", "up"));

    let path = "b/c/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Like `dotdot_even_other_in_middle_of_symlink`, but with a `/.` at the end.
///
/// Windows doesn't appear to like symlinks that end with `/.`.
#[test]
#[cfg_attr(windows, ignore)]
fn dotdot_slashdot_even_other_in_middle_of_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    let foo = b"foo";
    check!(h::create_dir_all(&start, "b/c"));
    check!(h::write(&start, "b/target", foo));
    let c = check!(p::open_dir(&start, Path::new("b/c")));
    check!(h::symlink_dir(&c, "../../b/.", "up"));

    let path = "b/c/up/target";
    let mut file = check!(h::open(&start, path));
    let mut data = Vec::new();
    check!(file.read_to_end(&mut data));
    assert_eq!(data, foo);
}

/// Ensure that a path of "/" is rejected.
#[test]
fn statat_slash() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    // FreeBSD 14+ uses `O_RESOLVE_BENEATH` which issues different errors.
    #[cfg(target_os = "freebsd")]
    {
        error_contains!(h::metadata(&start, "/"), "Capabilities insufficient");
        error_contains!(h::metadata(&start, "/foo"), "Capabilities insufficient");
        error_contains!(
            h::symlink_metadata(&start, "/"),
            "Capabilities insufficient"
        );
        error_contains!(
            h::symlink_metadata(&start, "/foo"),
            "Capabilities insufficient"
        );
    }

    #[cfg(not(target_os = "freebsd"))]
    {
        error_contains!(
            h::metadata(&start, "/"),
            "a path led outside of the filesystem"
        );
        error_contains!(
            h::metadata(&start, "/foo"),
            "a path led outside of the filesystem"
        );
        error_contains!(
            h::symlink_metadata(&start, "/"),
            "a path led outside of the filesyste"
        );
        error_contains!(
            h::symlink_metadata(&start, "/foo"),
            "a path led outside of the filesyste"
        );
    }
}

/// Test interactions between symlinks and trailing slashes.
#[test]
fn trailing_slash_symlink() {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);

    check!(h::create_dir(&start, "sandbox"));
    check!(h::symlink_dir(&start, "../outside", "sandbox/hidden"));
    check!(h::symlink_dir(&start, "hidden/", "sandbox/indirect"));

    let sandbox = check!(p::open_dir(&start, Path::new("sandbox")));

    for path in ["hidden", "hidden/", "indirect", "indirect/"] {
        error_contains!(
            p::open_dir(&sandbox, Path::new(path)),
            "a path led outside of the filesystem"
        );
        error_contains!(
            h::read_dir(&sandbox, path),
            "a path led outside of the filesystem"
        );
    }
}

/// Similar to `trailing_slash_symlink`, but populates the test directory
/// outside the sandbox, so it can cover more cases.
#[test]
fn trailing_slash_symlink_more() {
    let tmpdir = tempfile::tempdir().unwrap();

    check!(std::fs::create_dir(tmpdir.path().join("sandbox")));
    #[cfg(unix)]
    {
        check!(std::os::unix::fs::symlink(
            "../outside",
            tmpdir.path().join("sandbox/hidden")
        ));
        check!(std::os::unix::fs::symlink(
            "hidden/",
            tmpdir.path().join("sandbox/indirect")
        ));
        check!(std::os::unix::fs::symlink(
            "/.",
            tmpdir.path().join("sandbox/root_link")
        ));
    }
    #[cfg(windows)]
    {
        check!(std::os::windows::fs::symlink_dir(
            "../outside",
            tmpdir.path().join("sandbox/hidden")
        ));
        check!(std::os::windows::fs::symlink_dir(
            "hidden/",
            tmpdir.path().join("sandbox/indirect")
        ));
        check!(std::os::windows::fs::symlink_dir(
            "/.",
            tmpdir.path().join("sandbox/root_link")
        ));
    }
    #[cfg(not(any(unix, windows)))]
    {
        compile_error!("not implemented yet");
    }

    let start = check!(h::open_ambient_dir(tmpdir.path()));

    let sandbox = check!(p::open_dir(&start, Path::new("sandbox")));

    for path in [
        "hidden",
        "hidden/",
        "indirect",
        "indirect/",
        "root_link",
        "root_link/",
    ] {
        error_contains!(
            p::open_dir(&sandbox, Path::new(path)),
            "a path led outside of the filesystem"
        );
        error_contains!(
            h::read_dir(&sandbox, path),
            "a path led outside of the filesystem"
        );
    }
}
