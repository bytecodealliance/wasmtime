use std::{env, process};
use wasi_tests::{assert_errno, open_scratch_directory, TESTCONFIG};

unsafe fn test_dangling_symlink(dir_fd: wasi::Fd) {
    if TESTCONFIG.support_dangling_symlinks() {
        // First create a dangling symlink.
        wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

        // Try to open it as a directory with O_NOFOLLOW.
        assert_errno!(
            wasi::path_open(dir_fd, 0, "symlink", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
                .expect_err("opening a dangling symlink as a directory")
                .raw_error(),
            wasi::ERRNO_NOTDIR,
            wasi::ERRNO_LOOP
        );

        // Try to open it as a file with O_NOFOLLOW.
        assert_errno!(
            wasi::path_open(dir_fd, 0, "symlink", 0, 0, 0, 0)
                .expect_err("opening a dangling symlink as a file")
                .raw_error(),
            wasi::ERRNO_LOOP
        );

        // Clean up.
        wasi::path_unlink_file(dir_fd, "symlink").expect("failed to remove file");
    }
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
    unsafe { test_dangling_symlink(dir_fd) }
}
