use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_dangling_fd(dir_fd: wasi::Fd) {
    // Create a file, open it, delete it without closing the handle,
    // and then try creating it again
    let fd = wasi::path_open(dir_fd, 0, "file", wasi::OFLAGS_CREAT, 0, 0, 0).unwrap();
    wasi::fd_close(fd).unwrap();
    let file_fd = wasi::path_open(dir_fd, 0, "file", 0, 0, 0, 0).expect("failed to open");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    wasi::path_unlink_file(dir_fd, "file").expect("failed to unlink");
    let fd = wasi::path_open(dir_fd, 0, "file", wasi::OFLAGS_CREAT, 0, 0, 0).unwrap();
    wasi::fd_close(fd).unwrap();

    // Now, repeat the same process but for a directory
    wasi::path_create_directory(dir_fd, "subdir").expect("failed to create dir");
    let subdir_fd = wasi::path_open(dir_fd, 0, "subdir", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("failed to open dir");
    assert_gt!(
        subdir_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    wasi::path_remove_directory(dir_fd, "subdir").expect("failed to remove dir 2");
    wasi::path_create_directory(dir_fd, "subdir").expect("failed to create dir 2");
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
    unsafe { test_dangling_fd(dir_fd) }
}
