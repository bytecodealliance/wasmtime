use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd, drop_rights, fd_get_rights};
use wasi_tests::wasi_wrappers::{wasi_fd_read, wasi_path_open};

const TEST_FILENAME: &'static str = "file";

unsafe fn create_testfile(dir_fd: wasi_unstable::Fd) {
    let mut fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(
        dir_fd,
        0,
        TEST_FILENAME,
        wasi_unstable::O_CREAT | wasi_unstable::O_EXCL,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "creating a file"
    );
    close_fd(fd);
}

unsafe fn try_read_file(dir_fd: wasi_unstable::Fd) {
    let mut fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(dir_fd, 0, TEST_FILENAME, 0, 0, 0, 0, &mut fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening the test file"
    );

    // Check that we don't have the right to exeucute fd_read
    let (rbase, rinher) = fd_get_rights(fd);
    assert_eq!(rbase & wasi_unstable::RIGHT_FD_READ, 0, "should not have base RIGHT_FD_READ");
    assert_eq!(rinher & wasi_unstable::RIGHT_FD_READ, 0, "should not have inheriting RIGHT_FD_READ");

    let contents = &mut [0u8; 1];
    let iovec = wasi_unstable::IoVec {
        buf: contents.as_mut_ptr() as *mut libc::c_void,
        buf_len: contents.len(),
    };
    let mut nread = 0;
    // Since we no longer have the right to fd_read, trying to read a file
    // should be an error.
    let status = wasi_fd_read(fd, &[iovec], &mut nread);
    assert_ne!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading bytes from file"
    );
}

unsafe fn test_read_rights(dir_fd: wasi_unstable::Fd) {
    create_testfile(dir_fd);
    drop_rights(
        dir_fd,
        wasi_unstable::RIGHT_FD_READ,
        wasi_unstable::RIGHT_FD_READ,
    );

    let (rbase, rinher) = fd_get_rights(dir_fd);
    assert_eq!(rbase & wasi_unstable::RIGHT_FD_READ, 0, "dir should not have base RIGHT_FD_READ");
    assert_eq!(rinher & wasi_unstable::RIGHT_FD_READ, 0, "dir should not have inheriting RIGHT_FD_READ");

    try_read_file(dir_fd);
    cleanup_file(dir_fd, TEST_FILENAME);
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
