// This test module derived from Rust's
// src/test/ui-fulldeps/rename-directory.rs at revision
// 108e90ca78f052c0c1c49c42a22c85620be19712.

// run-pass

#![allow(unused_must_use)]
#![allow(unused_imports)]
// This test can't be a unit test in std,
// because it needs `TempDir`, which is in extra

// ignore-cross-compile

mod sys_common;

use cap_std::fs::{self, File};
use std::env;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use sys_common::io::tmpdir;

#[test]
fn rename_directory() {
    let tmpdir = tmpdir();
    let old_path = Path::new("foo/bar/baz");
    tmpdir.create_dir_all(&old_path).unwrap();
    let test_file = &old_path.join("temp.txt");

    tmpdir.create(test_file).unwrap();

    let new_path = Path::new("quux/blat");
    tmpdir.create_dir_all(&new_path).unwrap();
    tmpdir.rename(&old_path, &tmpdir, &new_path.join("newdir"));
    assert!(tmpdir.is_dir(new_path.join("newdir")));
    assert!(tmpdir.exists(new_path.join("newdir/temp.txt")));
}
