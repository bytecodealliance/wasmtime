#[macro_use]
mod sys_common;

use cap_std::ambient_authority;
use cap_std::fs::{Dir, File};

#[test]
fn test_open_ambient() {
    let _ = File::open_ambient("Cargo.toml", ambient_authority()).unwrap();
}

#[test]
fn test_create_ambient() {
    let dir = tempfile::tempdir().unwrap();
    let foo_path = dir.path().join("foo");
    let _ = File::create_ambient(&foo_path, ambient_authority()).unwrap();
    let _ = File::open_ambient(&foo_path, ambient_authority()).unwrap();
    let _ = File::create_ambient(&foo_path, ambient_authority()).unwrap();
}

#[test]
fn test_create_dir_ambient() {
    let dir = tempfile::tempdir().unwrap();
    let foo_path = dir.path().join("foo");
    Dir::create_ambient_dir_all(&foo_path, ambient_authority()).unwrap();
    let foo = Dir::open_ambient_dir(&foo_path, ambient_authority()).unwrap();
    let base = foo.open_parent_dir(ambient_authority()).unwrap();
    let _foo_again = base.open_dir("foo").unwrap();
}
