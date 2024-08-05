use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_file_allocate(dir_fd: wasi::Fd) {
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
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Check file size
    let mut stat = wasi::fd_filestat_get(file_fd).expect("reading file stats");
    assert_eq!(stat.size, 0, "file size should be 0");

    let err = wasi::fd_allocate(file_fd, 0, 100)
        .err()
        .expect("fd_allocate must fail");
    assert_eq!(
        err,
        wasi::ERRNO_NOTSUP,
        "fd_allocate should fail with NOTSUP"
    );

    stat = wasi::fd_filestat_get(file_fd).expect("reading file stats");
    assert_eq!(stat.size, 0, "file size should still be 0");

    wasi::fd_close(file_fd).expect("closing a file");
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
    unsafe { test_file_allocate(dir_fd) }
}
