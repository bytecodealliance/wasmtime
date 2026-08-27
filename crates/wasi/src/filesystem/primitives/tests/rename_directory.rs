// This test module derived from Rust's
// src/test/ui-fulldeps/rename-directory.rs at revision
// 108e90ca78f052c0c1c49c42a22c85620be19712.

// run-pass

#![allow(unused_must_use)]
// This test can't be a unit test in std,
// because it needs `TempDir`, which is in extra

// ignore-cross-compile

use super::helpers as h;
use super::sys_common::io::tmpdir;
use crate::filesystem::primitives::rename;
use std::path::Path;

#[test]
fn rename_directory() {
    let tmpdir = tmpdir();
    let dir = h::dir_of(&tmpdir);
    let old_path = Path::new("foo/bar/baz");
    h::create_dir_all(&dir, &old_path).unwrap();
    let test_file = &old_path.join("temp.txt");

    h::create(&dir, test_file).unwrap();

    let new_path = Path::new("quux/blat");
    h::create_dir_all(&dir, &new_path).unwrap();
    rename(&dir, &old_path, &dir, &new_path.join("newdir"));
    assert!(h::is_dir(&dir, &new_path.join("newdir")));
    assert!(h::exists(&dir, &new_path.join("newdir/temp.txt")));
}
