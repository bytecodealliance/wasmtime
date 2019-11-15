use libc;
use more_asserts::assert_gt;
use std::{env, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd};
use wasi_tests::wasi_wrappers::{wasi_fd_seek, wasi_fd_tell, wasi_fd_write, wasi_path_open};

unsafe fn test_file_seek_tell(dir_fd: wasi_unstable::Fd) {
    // Create a file in the scratch directory.
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
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

    // Check current offset
    let mut offset: wasi_unstable::FileSize = 0;
    status = wasi_fd_tell(file_fd, &mut offset);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "getting initial file offset"
    );
    assert_eq!(offset, 0, "current offset should be 0");

    // Write to file
    let buf = &[0u8; 100];
    let iov = wasi_unstable::CIoVec {
        buf: buf.as_ptr() as *const _,
        buf_len: buf.len(),
    };
    let iovs = &[iov];
    let mut nwritten = 0;
    status = wasi_fd_write(file_fd, iovs, &mut nwritten);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "writing to a file"
    );
    assert_eq!(nwritten, 100, "should write 100 bytes to file");

    // Check current offset
    status = wasi_fd_tell(file_fd, &mut offset);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "getting file offset after writing"
    );
    assert_eq!(offset, 100, "offset after writing should be 100");

    // Seek to middle of the file
    let mut newoffset = 1;
    status = wasi_fd_seek(file_fd, -50, wasi_unstable::WHENCE_CUR, &mut newoffset);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "seeking to the middle of a file"
    );
    assert_eq!(
        newoffset, 50,
        "offset after seeking to the middle should be at 50"
    );

    // Seek to the beginning of the file
    status = wasi_fd_seek(file_fd, 0, wasi_unstable::WHENCE_SET, &mut newoffset);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "seeking to the beginning of the file"
    );
    assert_eq!(
        newoffset, 0,
        "offset after seeking to the beginning of the file should be at 0"
    );

    // Seek beyond the file should be possible
    status = wasi_fd_seek(file_fd, 1000, wasi_unstable::WHENCE_CUR, &mut newoffset);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "seeking beyond the end of the file"
    );

    // Seek before byte 0 is an error though
    status = wasi_fd_seek(file_fd, -2000, wasi_unstable::WHENCE_CUR, &mut newoffset);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_EINVAL,
        "seeking before byte 0 is an error"
    );

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
    unsafe { test_file_seek_tell(dir_fd) }
}
