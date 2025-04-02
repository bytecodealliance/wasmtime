use std::{env, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_dirfd_not_dir(dir_fd: wasip1::Fd) {
    // Open a file.
    let file_fd = wasip1::path_open(dir_fd, 0, "file", wasip1::OFLAGS_CREAT, 0, 0, 0)
        .expect("opening a file");
    // Now try to open a file underneath it as if it were a directory.
    assert_errno!(
        wasip1::path_open(file_fd, 0, "foo", wasip1::OFLAGS_CREAT, 0, 0, 0)
            .expect_err("non-directory base fd should get ERRNO_NOTDIR"),
        wasip1::ERRNO_NOTDIR
    );
    wasip1::fd_close(file_fd).expect("closing a file");
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
    unsafe { test_dirfd_not_dir(dir_fd) }
}
