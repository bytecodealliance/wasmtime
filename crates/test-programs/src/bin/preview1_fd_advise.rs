use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_fd_advise(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("failed to open file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Check file size
    let stat = wasi::fd_filestat_get(file_fd).expect("failed to fdstat");
    assert_eq!(stat.size, 0, "file size should be 0");

    // set_size it bigger
    wasi::fd_filestat_set_size(file_fd, 100).expect("setting size");

    let stat = wasi::fd_filestat_get(file_fd).expect("failed to fdstat 2");
    assert_eq!(stat.size, 100, "file size should be 100");

    // Advise the kernel
    wasi::fd_advise(file_fd, 10, 50, wasi::ADVICE_NORMAL).expect("failed advise");

    // Advise shouldn't change size
    let stat = wasi::fd_filestat_get(file_fd).expect("failed to fdstat 3");
    assert_eq!(stat.size, 100, "file size should be 100");

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
