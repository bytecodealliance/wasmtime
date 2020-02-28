use std::{env, process};
use wasi_tests::open_scratch_directory;
use wasi_tests::{create_file, drop_rights, fd_get_rights};

const TEST_FILENAME: &'static str = "file";

unsafe fn try_read_file(dir_fd: wasi::Fd) {
    let fd = wasi::path_open(dir_fd, 0, TEST_FILENAME, 0, 0, 0, 0).expect("opening the file");

    // Check that we don't have the right to exeucute fd_read
    let (rbase, rinher) = fd_get_rights(fd);
    assert_eq!(
        rbase & wasi::RIGHTS_FD_READ,
        0,
        "should not have base RIGHTS_FD_READ"
    );
    assert_eq!(
        rinher & wasi::RIGHTS_FD_READ,
        0,
        "should not have inheriting RIGHTS_FD_READ"
    );

    let contents = &mut [0u8; 1];
    let iovec = wasi::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    // Since we no longer have the right to fd_read, trying to read a file
    // should be an error.
    assert_eq!(
        wasi::fd_read(fd, &[iovec])
            .expect_err("reading bytes from file should fail")
            .raw_error(),
        wasi::ERRNO_NOTCAPABLE,
        "the errno should be ENOTCAPABLE"
    );
}

unsafe fn test_read_rights(dir_fd: wasi::Fd) {
    create_file(dir_fd, TEST_FILENAME);
    drop_rights(dir_fd, wasi::RIGHTS_FD_READ, wasi::RIGHTS_FD_READ);

    let (rbase, rinher) = fd_get_rights(dir_fd);
    assert_eq!(
        rbase & wasi::RIGHTS_FD_READ,
        0,
        "dir should not have base RIGHTS_FD_READ"
    );
    assert_eq!(
        rinher & wasi::RIGHTS_FD_READ,
        0,
        "dir should not have inheriting RIGHTS_FD_READ"
    );

    try_read_file(dir_fd);
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {} <scratch directory>", prog);
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe { test_read_rights(dir_fd) }
}
