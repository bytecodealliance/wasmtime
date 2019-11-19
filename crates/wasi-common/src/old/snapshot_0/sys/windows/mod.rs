pub(crate) mod fdentry_impl;
pub(crate) mod host_impl;
pub(crate) mod hostcalls_impl;

use crate::old::snapshot_0::Result;
use std::fs::{File, OpenOptions};
use std::path::Path;

pub(crate) fn dev_null() -> Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("NUL")
        .map_err(Into::into)
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> Result<File> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS;

    // To open a directory using CreateFile, specify the
    // FILE_FLAG_BACKUP_SEMANTICS flag as part of dwFileFlags...
    // cf. https://docs.microsoft.com/en-us/windows/desktop/api/fileapi/nf-fileapi-createfile2
    OpenOptions::new()
        .create(false)
        .write(true)
        .read(true)
        .attributes(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .map_err(Into::into)
}
