use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_file_seek_tell(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE | wasi::RIGHTS_FD_SEEK | wasi::RIGHTS_FD_TELL,
        0,
        0,
    )
    .expect("opening a file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Check current offset
    let mut offset = wasi::fd_tell(file_fd).expect("getting initial file offset");
    assert_eq!(offset, 0, "current offset should be 0");

    // Write to file
    let data = &[0u8; 100];
    let iov = wasi::Ciovec {
        buf: data.as_ptr() as *const _,
        buf_len: data.len(),
    };
    let nwritten = wasi::fd_write(file_fd, &[iov]).expect("writing to a file");
    assert_eq!(nwritten, 100, "should write 100 bytes to file");

    // Check current offset
    offset = wasi::fd_tell(file_fd).expect("getting file offset after writing");
    assert_eq!(offset, 100, "offset after writing should be 100");

    // Seek to middle of the file
    let mut newoffset =
        wasi::fd_seek(file_fd, -50, wasi::WHENCE_CUR).expect("seeking to the middle of a file");
    assert_eq!(
        newoffset, 50,
        "offset after seeking to the middle should be at 50"
    );

    // Seek to the beginning of the file
    newoffset =
        wasi::fd_seek(file_fd, 0, wasi::WHENCE_SET).expect("seeking to the beginning of the file");
    assert_eq!(
        newoffset, 0,
        "offset after seeking to the beginning of the file should be at 0"
    );

    // Seek beyond the file should be possible
    wasi::fd_seek(file_fd, 1000, wasi::WHENCE_CUR).expect("seeking beyond the end of the file");

    // Seek before byte 0 is an error though
    assert_eq!(
        wasi::fd_seek(file_fd, -2000, wasi::WHENCE_CUR)
            .expect_err("seeking before byte 0 should be an error")
            .raw_error(),
        wasi::ERRNO_INVAL,
        "errno should be ERRNO_INVAL",
    );

    // Check that fd_read properly updates the file offset
    wasi::fd_seek(file_fd, 0, wasi::WHENCE_SET).expect("seeking to the beginning of the file again");

    let buffer = &mut [0u8; 100];
    let iovec = wasi::Iovec {
        buf: buffer.as_mut_ptr(),
        buf_len: buffer.len(),
    };
    let nread = wasi::fd_read(file_fd, &[iovec]).expect("reading file");
    assert_eq!(nread, buffer.len(), "should read {} bytes", buffer.len());

    offset = wasi::fd_tell(file_fd).expect("getting file offset after reading");
    assert_eq!(offset, 100, "offset after reading should be 100");

    wasi::fd_close(file_fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "file").expect("deleting a file");
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
