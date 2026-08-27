// This file contains tests for `cap_fs_ext::FileTypeExt`.

#![cfg_attr(all(windows, windows_by_handle), feature(windows_by_handle))]

#[macro_use]
mod sys_common;

use sys_common::io::tmpdir;

#[test]
fn test_dir_entry_ext() {
    let tmpdir = tmpdir();
    check!(tmpdir.create("a"));

    // First try with the regular `metadata()`. All nones.
    #[cfg(all(windows, windows_by_handle))]
    for entry in check!(tmpdir.entries()) {
        use cap_std::fs::MetadataExt;
        let entry = check!(entry);
        assert!(check!(entry.metadata()).volume_serial_number().is_none());
        assert!(check!(entry.metadata()).number_of_links().is_none());
        assert!(check!(entry.metadata()).file_index().is_none());
    }

    // Now try with `full_metadata()`.
    for entry in check!(tmpdir.entries()) {
        use cap_fs_ext::{DirEntryExt, MetadataExt};
        let entry = check!(entry);
        check!(entry.full_metadata()).dev();
        check!(entry.full_metadata()).nlink();
        check!(entry.full_metadata()).ino();
    }
}
