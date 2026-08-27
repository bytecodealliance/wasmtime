// This file contains tests for `FileTypeExt`.

use super::sys_common::io::tmpdir;
#[cfg(unix)]
use crate::filesystem::primitives::FileTypeExt;
use crate::filesystem::primitives::{Metadata, OpenOptions, open, open_ambient_dir};
use ambient_authority::ambient_authority;
use std::path::Path;

#[test]
fn test_file_type_ext() {
    let tmpdir = tmpdir();
    let dir = check!(open_ambient_dir(tmpdir.path(), ambient_authority()));
    let a = check!(open(
        &dir,
        Path::new("a"),
        OpenOptions::new().write(true).create(true).truncate(true)
    ));

    let tmpdir_metadata = check!(Metadata::from_file(&dir));
    let a_metadata = check!(Metadata::from_file(&a));

    #[cfg(unix)]
    {
        assert!(!tmpdir_metadata.file_type().is_char_device());
        assert!(!a_metadata.file_type().is_char_device());

        assert!(!tmpdir_metadata.file_type().is_block_device());
        assert!(!a_metadata.file_type().is_block_device());
    }

    assert!(tmpdir_metadata.file_type().is_dir());
    assert!(!a_metadata.file_type().is_dir());

    assert!(!tmpdir_metadata.file_type().is_file());
    assert!(a_metadata.file_type().is_file());

    assert!(!tmpdir_metadata.file_type().is_symlink());
    assert!(!a_metadata.file_type().is_symlink());
}
