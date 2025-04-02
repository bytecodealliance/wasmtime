use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_file_allocate(dir_fd: wasip1::Fd) {
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

    // Check file size
    let mut stat = wasip1::fd_filestat_get(file_fd).expect("reading file stats");
    assert_eq!(stat.size, 0, "file size should be 0");

    let err = wasip1::fd_allocate(file_fd, 0, 100)
        .err()
        .expect("fd_allocate must fail");
    assert_eq!(
        err,
        wasip1::ERRNO_NOTSUP,
        "fd_allocate should fail with NOTSUP"
    );

    stat = wasip1::fd_filestat_get(file_fd).expect("reading file stats");
    assert_eq!(stat.size, 0, "file size should still be 0");

    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_file_allocate(dir_fd) }
}
