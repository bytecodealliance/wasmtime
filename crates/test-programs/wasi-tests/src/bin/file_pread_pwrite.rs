use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_file_pread_pwrite(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_SEEK | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    let contents = &[0u8, 1, 2, 3];
    let ciovec = wasi::Ciovec {
        buf: contents.as_ptr() as *const _,
        buf_len: contents.len(),
    };
    let mut nwritten =
        wasi::fd_pwrite(file_fd, &mut [ciovec], 0).expect("writing bytes at offset 0");
    assert_eq!(nwritten, 4, "nwritten bytes check");

    let contents = &mut [0u8; 4];
    let iovec = wasi::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    let mut nread = wasi::fd_pread(file_fd, &[iovec], 0).expect("reading bytes at offset 0");
    assert_eq!(nread, 4, "nread bytes check");
    assert_eq!(contents, &[0u8, 1, 2, 3], "written bytes equal read bytes");

    let contents = &mut [0u8; 4];
    let iovec = wasi::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    nread = wasi::fd_pread(file_fd, &[iovec], 2).expect("reading bytes at offset 2");
    assert_eq!(nread, 2, "nread bytes check");
    assert_eq!(contents, &[2u8, 3, 0, 0], "file cursor was overwritten");

    let contents = &[1u8, 0];
    let ciovec = wasi::Ciovec {
        buf: contents.as_ptr() as *const _,
        buf_len: contents.len(),
    };
    nwritten = wasi::fd_pwrite(file_fd, &mut [ciovec], 2).expect("writing bytes at offset 2");
    assert_eq!(nwritten, 2, "nwritten bytes check");

    let contents = &mut [0u8; 4];
    let iovec = wasi::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    nread = wasi::fd_pread(file_fd, &[iovec], 0).expect("reading bytes at offset 0");
    assert_eq!(nread, 4, "nread bytes check");
    assert_eq!(contents, &[0u8, 1, 1, 0], "file cursor was overwritten");

    wasi::fd_close(file_fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "file").expect("removing a file");
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
