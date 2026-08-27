//! Tests for various forms of reopening a directory handle.

#[macro_use]
mod sys_common;

use sys_common::io::tmpdir;

#[test]
fn reopendir_a() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));

    let inner = check!(tmpdir.open_dir("dir/inner"));

    check!(inner.open_dir("."));
}

#[test]
fn reopendir_b() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));

    let inner = check!(tmpdir.open_dir("dir/inner"));

    check!(inner.open_dir("./"));
}

#[test]
fn reopendir_c() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));

    let inner = check!(tmpdir.open_dir("dir/inner"));

    check!(inner.open_dir("./."));
}

#[test]
fn reopendir_d() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));

    let _inner = check!(tmpdir.open_dir("dir/inner"));

    check!(tmpdir.open_dir("dir/inner"));
}

#[test]
fn reopendir_e() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));

    let _inner = check!(tmpdir.open_dir("dir/inner"));

    check!(tmpdir.open_dir("dir/inner/."));
}

#[test]
fn reopendir_f() {
    let tmpdir = tmpdir();
    check!(tmpdir.create_dir_all("dir/inner"));

    let _inner = check!(tmpdir.open_dir("dir/inner"));

    check!(tmpdir.open_dir("dir/inner/"));
}
