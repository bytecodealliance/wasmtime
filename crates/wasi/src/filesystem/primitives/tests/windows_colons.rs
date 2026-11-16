#![cfg(windows)]

use super::helpers as h;
use super::sys_common::io::tmpdir;
use super::sys_common::symlink_supported;
use crate::filesystem::primitives as p;
use std::path::Path;

const PATHS: &[&str] = &[
    "C:",
    "C:x",
    "./C:",
    "./C:x",
    "./C:/x",
    "./C:\\x",
    "./D:x",
    "./c:x",
    "dir/C:",
    "dir/C:x",
    "dir/C:/x",
    "dir/../C:x",
    "file:stream",
    "file::$DATA",
    "file:stream:$DATA",
    "dir/file:stream",
    "dir:stream",
    "dir:stream/file",
];

fn setup() -> (tempfile::TempDir, std::fs::File) {
    let tmpdir = tmpdir();
    let start = h::dir_of(&tmpdir);
    check!(h::create(&start, "file"));
    check!(h::create_dir(&start, "dir"));
    check!(h::create(&start, "dir/file"));
    (tmpdir, start)
}

#[test]
fn colon_open() {
    let (_tmpdir, start) = setup();
    for path in PATHS {
        assert!(h::open(&start, path).is_err(), "open {path:?}");
        assert!(h::create(&start, path).is_err(), "create {path:?}");
        assert!(
            p::open_dir(&start, Path::new(path)).is_err(),
            "open_dir {path:?}"
        );
    }

    // Sanity check that normal paths still work.
    check!(h::open(&start, "file"));
    check!(h::open(&start, "dir/file"));
    check!(h::open(&start, "dir/../file"));
}

#[test]
fn colon_stat() {
    let (_tmpdir, start) = setup();
    for path in PATHS {
        for follow in [p::FollowSymlinks::Yes, p::FollowSymlinks::No] {
            assert!(
                p::stat(&start, Path::new(path), follow).is_err(),
                "stat {path:?} {follow:?}"
            );
        }
    }
}

#[test]
fn colon_parent_ops() {
    let (_tmpdir, start) = setup();
    for path in PATHS {
        let path = Path::new(path);
        assert!(h::create_dir(&start, path).is_err(), "create_dir {path:?}");
        assert!(
            p::remove_file(&start, path).is_err(),
            "remove_file {path:?}"
        );
        assert!(p::remove_dir(&start, path).is_err(), "remove_dir {path:?}");
        assert!(
            p::rename(&start, Path::new("file"), &start, path).is_err(),
            "rename to {path:?}"
        );
        assert!(
            p::rename(&start, path, &start, Path::new("renamed")).is_err(),
            "rename from {path:?}"
        );
        assert!(
            p::hard_link(&start, Path::new("file"), &start, path).is_err(),
            "hard_link to {path:?}"
        );
        assert!(
            p::hard_link(&start, path, &start, Path::new("linked")).is_err(),
            "hard_link from {path:?}"
        );
        assert!(
            p::set_times(&start, path, None, None).is_err(),
            "set_times {path:?}"
        );
        assert!(p::read_link(&start, path).is_err(), "read_link {path:?}");
    }

    // Nothing above should have touched the original files.
    assert!(h::is_file(&start, "file"));
    assert!(h::is_file(&start, "dir/file"));
    assert!(!h::exists(&start, "renamed"));
    assert!(!h::exists(&start, "linked"));
}

#[test]
fn colon_symlinks() {
    if !symlink_supported() {
        return;
    }
    let (tmpdir, start) = setup();

    // Creating a symlink whose link name contains a colon is rejected.
    for path in PATHS {
        assert!(
            p::symlink_file(Path::new("file"), &start, Path::new(path)).is_err(),
            "symlink_file link {path:?}"
        );
    }

    // Symlinks whose targets contain a colon can't be followed, whether the
    // guest or someone else created them.
    for (i, target) in ["dir/C:", "dir/C:/x", "dir/C:x", "file:stream"]
        .iter()
        .enumerate()
    {
        let guest = format!("guest_link{i}");
        let host = format!("host_link{i}");
        let _ = p::symlink_file(Path::new(target), &start, Path::new(&guest));
        check!(std::os::windows::fs::symlink_file(
            target,
            tmpdir.path().join(&host)
        ));
        for name in [&guest, &host] {
            assert!(h::open(&start, name).is_err(), "open {name} -> {target:?}");
            assert!(
                h::open(&start, format!("{name}/file")).is_err(),
                "open {name}/file -> {target:?}"
            );
            assert!(
                p::stat(&start, Path::new(name), p::FollowSymlinks::Yes).is_err(),
                "stat {name} -> {target:?}"
            );
        }
    }
}
