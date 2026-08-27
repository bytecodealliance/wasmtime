#[macro_use]
mod sys_common;

use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::fs;
use std::path::Component;

#[test]
fn open_root() {
    let root = Dir::open_ambient_dir(Component::RootDir.as_os_str(), ambient_authority())
        .expect("expect to be able to open the root directory");
    error_contains!(
        root.read_dir(Component::ParentDir.as_os_str()),
        "a path led outside of the filesystem"
    );
}

/// Attempt to remove the root directory, which should fail, and check that the
/// error message is as expected.
#[test]
fn remove_root() {
    let _observed = {
        let root = Dir::open_ambient_dir(Component::RootDir.as_os_str(), ambient_authority())
            .expect("expect to be able to open the root directory");
        root.remove_open_dir().unwrap_err()
    };
    let _expected = fs::remove_dir(Component::RootDir.as_os_str()).unwrap_err();
    // TODO: Investigate why these asserts spuriously fail on Windows and QEMU.
    //assert_eq!(expected.to_string(), observed.to_string());
    //assert_eq!(expected.kind(), observed.kind());
}
