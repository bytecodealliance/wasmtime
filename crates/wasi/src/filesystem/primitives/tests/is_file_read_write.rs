#[macro_use]
mod sys_common;

use cap_fs_ext::{IsFileReadWrite, OpenOptions};
use sys_common::io::tmpdir;

#[test]
fn basic_is_file_read_write() {
    let tmpdir = tmpdir();

    let file = check!(tmpdir.open_with(
        "file",
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
    ));
    assert_eq!(check!(file.is_file_read_write()), (true, true));

    let file = check!(tmpdir.open_with("file", OpenOptions::new().append(true).read(true)));
    assert_eq!(check!(file.is_file_read_write()), (true, true));

    let file = check!(tmpdir.open_with(
        "file",
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(false)
    ));
    assert_eq!(check!(file.is_file_read_write()), (false, true));

    let file = check!(tmpdir.open_with(
        "file",
        OpenOptions::new()
            .create(false)
            .truncate(false)
            .write(false)
            .read(true)
    ));
    assert_eq!(check!(file.is_file_read_write()), (true, false));
}
