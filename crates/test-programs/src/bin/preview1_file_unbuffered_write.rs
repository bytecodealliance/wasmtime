use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_file_unbuffered_write(dir_fd: wasi::Fd) {
    // Create and open file for reading
    let fd_read = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ,
        0,
        0,
    )
    .expect("create and open file for reading");
    assert!(
        fd_read > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Open the same file but for writing
    let fd_write = wasi::path_open(dir_fd, 0, "file", 0, wasi::RIGHTS_FD_WRITE, 0, 0)
        .expect("opening file for writing");
    assert!(
        fd_write > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Write to file
    let contents = &[1u8];
    let ciovec = wasi::Ciovec {
        buf: contents.as_ptr() as *const _,
        buf_len: contents.len(),
    };
    let nwritten = wasi::fd_write(fd_write, &[ciovec]).expect("writing byte to file");
    assert_eq!(nwritten, 1, "nwritten bytes check");

    // Read from file
    let contents = &mut [0u8; 1];
    let iovec = wasi::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    let nread = wasi::fd_read(fd_read, &[iovec]).expect("reading bytes from file");
    assert_eq!(nread, 1, "nread bytes check");
    assert_eq!(contents, &[1u8], "written bytes equal read bytes");

    // Clean up
    wasi::fd_close(fd_write).expect("closing a file");
    wasi::fd_close(fd_read).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_file_unbuffered_write(dir_fd) }
}
