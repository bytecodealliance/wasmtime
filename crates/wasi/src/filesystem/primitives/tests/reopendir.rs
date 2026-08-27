//! Tests for various forms of reopening a directory handle.

use super::helpers as h;
use super::sys_common::io::tmpdir;
use crate::filesystem::primitives::open_dir;
use std::path::Path;

#[test]
fn reopendir_a() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));

    let inner = check!(open_dir(&dir, Path::new("dir/inner")));

    check!(open_dir(&inner, Path::new(".")));
}

#[test]
fn reopendir_b() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));

    let inner = check!(open_dir(&dir, Path::new("dir/inner")));

    check!(open_dir(&inner, Path::new("./")));
}

#[test]
fn reopendir_c() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));

    let inner = check!(open_dir(&dir, Path::new("dir/inner")));

    check!(open_dir(&inner, Path::new("./.")));
}

#[test]
fn reopendir_d() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));

    let _inner = check!(open_dir(&dir, Path::new("dir/inner")));

    check!(open_dir(&dir, Path::new("dir/inner")));
}

#[test]
fn reopendir_e() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));

    let _inner = check!(open_dir(&dir, Path::new("dir/inner")));

    check!(open_dir(&dir, Path::new("dir/inner/.")));
}

#[test]
fn reopendir_f() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    check!(h::create_dir_all(&dir, "dir/inner"));

    let _inner = check!(open_dir(&dir, Path::new("dir/inner")));

    check!(open_dir(&dir, Path::new("dir/inner/")));
}
