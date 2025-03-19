use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_fd_advise(dir_fd: wasip1::Fd) {
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
    .expect("failed to open file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    // Check file size
    let stat = wasip1::fd_filestat_get(file_fd).expect("failed to fdstat");
    assert_eq!(stat.size, 0, "file size should be 0");

    // set_size it bigger
    wasip1::fd_filestat_set_size(file_fd, 100).expect("setting size");

    let stat = wasip1::fd_filestat_get(file_fd).expect("failed to fdstat 2");
    assert_eq!(stat.size, 100, "file size should be 100");

    // Advise the kernel
    wasip1::fd_advise(file_fd, 10, 50, wasip1::ADVICE_NORMAL).expect("failed advise");

    // Advise shouldn't change size
    let stat = wasip1::fd_filestat_get(file_fd).expect("failed to fdstat 3");
    assert_eq!(stat.size, 100, "file size should be 100");

    wasip1::fd_close(file_fd).expect("failed to close");
    wasip1::path_unlink_file(dir_fd, "file").expect("failed to unlink");
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
    unsafe { test_fd_advise(dir_fd) }
}
