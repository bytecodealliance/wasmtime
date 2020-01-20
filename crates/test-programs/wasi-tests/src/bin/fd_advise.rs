use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_fd_advise(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ
            | wasi::RIGHTS_FD_WRITE
            | wasi::RIGHTS_FD_ADVISE
            | wasi::RIGHTS_FD_FILESTAT_GET
            | wasi::RIGHTS_FD_ALLOCATE,
        0,
        0,
    )
    .expect("failed to open file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Check file size
    let stat = wasi::fd_filestat_get(file_fd).expect("failed to fdstat");
    assert_eq!(stat.size, 0, "file size should be 0");

    // Allocate some size
    wasi::fd_allocate(file_fd, 0, 100).expect("allocating size");

    let stat = wasi::fd_filestat_get(file_fd).expect("failed to fdstat 2");
    assert_eq!(stat.size, 100, "file size should be 100");

    // Advise the kernel
    wasi::fd_advise(file_fd, 10, 50, wasi::ADVICE_NORMAL).expect("failed advise");

    wasi::fd_close(file_fd).expect("failed to close");
    wasi::path_unlink_file(dir_fd, "file").expect("failed to unlink");
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
    unsafe { test_fd_advise(dir_fd) }
}
