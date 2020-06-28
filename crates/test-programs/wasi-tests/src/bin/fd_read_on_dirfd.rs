use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_fd_read_on_dirfd(dir_fd: wasi::Fd) {
    // Create a directory.
    wasi::path_create_directory(dir_fd, "subdir").expect("creating subdirectory");

    // Open it as dir
    let fd = wasi::path_open(
        dir_fd,
        0,
        "subdir",
        wasi::OFLAGS_DIRECTORY,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_SEEK,
        0,
        0,
    )
    .expect("open subdir for reading");
    // Try reading from it
    let contents = &mut [0u8; 4];
    let iovec = wasi::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    assert_eq!(
        wasi::fd_read(fd, &[iovec])
            .expect_err("calling fd_read on directory should get ERRNO_ISDIR")
            .raw_error(),
        wasi::ERRNO_ISDIR,
        "errno should be ERRNO_ISDIR"
    );
    assert_eq!(
        wasi::fd_pread(fd, &[iovec], 0)
            .expect_err("calling fd_pread on directory should get ERRNO_ISDIR")
            .raw_error(),
        wasi::ERRNO_ISDIR,
        "errno should be ERRNO_ISDIR"
    );
    wasi::fd_close(fd).expect("closing an fd");

    // Now, open as file
    let fd = wasi::path_open(dir_fd, 0, "subdir", 0, wasi::RIGHTS_FD_READ, 0, 0)
        .expect("open subdir as file for reading");
    // Try reading from it
    let contents = &mut [0u8; 4];
    let iovec = wasi::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    assert_eq!(
        wasi::fd_read(fd, &[iovec])
            .expect_err("calling fd_read on directory should get ERRNO_ISDIR")
            .raw_error(),
        wasi::ERRNO_ISDIR,
        "errno should be ERRNO_ISDIR"
    );
    assert_eq!(
        wasi::fd_pread(fd, &[iovec], 0)
            .expect_err("calling fd_pread on directory should get ERRNO_ISDIR")
            .raw_error(),
        wasi::ERRNO_ISDIR,
        "errno should be ERRNO_ISDIR"
    );
    wasi::fd_close(fd).expect("closing an fd");
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
    unsafe { test_fd_read_on_dirfd(dir_fd) }
}
