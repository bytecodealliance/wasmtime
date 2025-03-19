use std::{env, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_file_seek_tell(dir_fd: wasip1::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        "file",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    // Check current offset
    let mut offset = wasip1::fd_tell(file_fd).expect("getting initial file offset");
    assert_eq!(offset, 0, "current offset should be 0");

    // Write to file
    let data = &[0u8; 100];
    let iov = wasip1::Ciovec {
        buf: data.as_ptr() as *const _,
        buf_len: data.len(),
    };
    let nwritten = wasip1::fd_write(file_fd, &[iov]).expect("writing to a file");
    assert_eq!(nwritten, 100, "should write 100 bytes to file");

    // Check current offset
    offset = wasip1::fd_tell(file_fd).expect("getting file offset after writing");
    assert_eq!(offset, 100, "offset after writing should be 100");

    // Seek to middle of the file
    let mut newoffset =
        wasip1::fd_seek(file_fd, -50, wasip1::WHENCE_CUR).expect("seeking to the middle of a file");
    assert_eq!(
        newoffset, 50,
        "offset after seeking to the middle should be at 50"
    );

    // Seek to the beginning of the file
    newoffset = wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_SET)
        .expect("seeking to the beginning of the file");
    assert_eq!(
        newoffset, 0,
        "offset after seeking to the beginning of the file should be at 0"
    );

    // Seek beyond the file should be possible
    wasip1::fd_seek(file_fd, 1000, wasip1::WHENCE_CUR).expect("seeking beyond the end of the file");

    // Seek before byte 0 is an error though
    assert_errno!(
        wasip1::fd_seek(file_fd, -2000, wasip1::WHENCE_CUR)
            .expect_err("seeking before byte 0 should be an error"),
        wasip1::ERRNO_INVAL
    );

    // Check that fd_read properly updates the file offset
    wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_SET)
        .expect("seeking to the beginning of the file again");

    let buffer = &mut [0u8; 100];
    let iovec = wasip1::Iovec {
        buf: buffer.as_mut_ptr(),
        buf_len: buffer.len(),
    };
    let nread = wasip1::fd_read(file_fd, &[iovec]).expect("reading file");
    assert_eq!(nread, buffer.len(), "should read {} bytes", buffer.len());

    offset = wasip1::fd_tell(file_fd).expect("getting file offset after reading");
    assert_eq!(offset, 100, "offset after reading should be 100");

    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, "file").expect("deleting a file");
}

// Test that when a file is opened with `O_APPEND` that acquiring the current
// position indicates the end of the file.
unsafe fn seek_and_o_append(dir_fd: wasip1::Fd) {
    let path = "file2";
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        path,
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        wasip1::FDFLAGS_APPEND,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    let mut offset = wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_CUR).unwrap();
    assert_eq!(offset, 0);
    offset = wasip1::fd_tell(file_fd).unwrap();
    assert_eq!(offset, 0);

    let data = &[0u8; 100];
    let iov = wasip1::Ciovec {
        buf: data.as_ptr() as *const _,
        buf_len: data.len(),
    };
    let nwritten = wasip1::fd_write(file_fd, &[iov]).unwrap();
    assert_eq!(nwritten, 100);

    let mut offset = wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_CUR).unwrap();
    assert_eq!(offset, 100);
    offset = wasip1::fd_tell(file_fd).unwrap();
    assert_eq!(offset, 100);

    wasip1::fd_close(file_fd).unwrap();
    wasip1::path_unlink_file(dir_fd, path).unwrap();
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {prog} <scratch directory>");
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe {
        test_file_seek_tell(dir_fd);
        seek_and_o_append(dir_fd);
    }
}
