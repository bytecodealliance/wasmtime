pub mod fdentry;
mod host_impl;
pub mod hostcalls;

use std::fs::File;
use std::io;
use std::path::Path;

pub fn dev_null() -> File {
    File::open("NUL").expect("failed to open NUL")
}

pub fn preopen_dir<P: AsRef<Path>>(path: P) -> io::Result<File> {
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
}
