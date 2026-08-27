// This file contains tests for `cap_fs_ext::FileTypeExt`.

#[macro_use]
mod sys_common;

use cap_fs_ext::FileTypeExt;
use sys_common::io::tmpdir;

#[test]
fn test_file_type_ext() {
    let tmpdir = tmpdir();
    let a = check!(tmpdir.create("a"));

    let tmpdir_metadata = check!(tmpdir.dir_metadata());
    let a_metadata = check!(a.metadata());

    assert!(!tmpdir_metadata.file_type().is_char_device());
    assert!(!a_metadata.file_type().is_char_device());

    assert!(!tmpdir_metadata.file_type().is_block_device());
    assert!(!a_metadata.file_type().is_block_device());

    assert!(!tmpdir_metadata.file_type().is_fifo());
    assert!(!a_metadata.file_type().is_fifo());

    assert!(!tmpdir_metadata.file_type().is_socket());
    assert!(!a_metadata.file_type().is_socket());

    assert!(tmpdir_metadata.file_type().is_dir());
    assert!(!a_metadata.file_type().is_dir());

    assert!(!tmpdir_metadata.file_type().is_file());
    assert!(a_metadata.file_type().is_file());

    assert!(!tmpdir_metadata.file_type().is_symlink());
    assert!(!a_metadata.file_type().is_symlink());
}
