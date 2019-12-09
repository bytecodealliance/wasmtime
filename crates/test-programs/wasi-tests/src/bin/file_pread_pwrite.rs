use libc;
use more_asserts::assert_gt;
use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd};
use wasi_tests::wasi_wrappers::{wasi_fd_pread, wasi_fd_pwrite, wasi_path_open};

unsafe fn test_file_pread_pwrite(dir_fd: wasi_unstable::Fd) {
    // Create a file in the scratch directory.
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_SEEK | wasi_unstable::RIGHT_FD_WRITE,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );

    let contents = &[0u8, 1, 2, 3];
    let ciovec = wasi_unstable::CIoVec {
        buf: contents.as_ptr() as *const libc::c_void,
        buf_len: contents.len(),
    };
    let mut nwritten = 0;
    status = wasi_fd_pwrite(file_fd, &mut [ciovec], 0, &mut nwritten);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "writing bytes at offset 0"
    );
    assert_eq!(nwritten, 4, "nwritten bytes check");

    let contents = &mut [0u8; 4];
    let iovec = wasi_unstable::IoVec {
        buf: contents.as_mut_ptr() as *mut libc::c_void,
        buf_len: contents.len(),
    };
    let mut nread = 0;
    status = wasi_fd_pread(file_fd, &[iovec], 0, &mut nread);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading bytes at offset 0"
    );
    assert_eq!(nread, 4, "nread bytes check");
    assert_eq!(contents, &[0u8, 1, 2, 3], "written bytes equal read bytes");

    let contents = &mut [0u8; 4];
    let iovec = wasi_unstable::IoVec {
        buf: contents.as_mut_ptr() as *mut libc::c_void,
        buf_len: contents.len(),
    };
    let mut nread = 0;
    status = wasi_fd_pread(file_fd, &[iovec], 2, &mut nread);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading bytes at offset 2"
    );
    assert_eq!(nread, 2, "nread bytes check");
    assert_eq!(contents, &[2u8, 3, 0, 0], "file cursor was overwritten");

    let contents = &[1u8, 0];
    let ciovec = wasi_unstable::CIoVec {
        buf: contents.as_ptr() as *const libc::c_void,
        buf_len: contents.len(),
    };
    let mut nwritten = 0;
    status = wasi_fd_pwrite(file_fd, &mut [ciovec], 2, &mut nwritten);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "writing bytes at offset 2"
    );
    assert_eq!(nwritten, 2, "nwritten bytes check");

    let contents = &mut [0u8; 4];
    let iovec = wasi_unstable::IoVec {
        buf: contents.as_mut_ptr() as *mut libc::c_void,
        buf_len: contents.len(),
    };
    let mut nread = 0;
    status = wasi_fd_pread(file_fd, &[iovec], 0, &mut nread);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading bytes at offset 0"
    );
    assert_eq!(nread, 4, "nread bytes check");
    assert_eq!(contents, &[0u8, 1, 1, 0], "file cursor was overwritten");

    close_fd(file_fd);
    cleanup_file(dir_fd, "file");
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
    unsafe { test_file_pread_pwrite(dir_fd) }
}
