use std::{env, process};
use wasi_tests::{assert_errno, open_scratch_directory};

unsafe fn test_path_open_missing(dir_fd: wasi::Fd) {
    assert_errno!(
        wasi::path_open(
            dir_fd, 0, "file", 0, // not passing O_CREAT here
            0, 0, 0,
        )
        .expect_err("trying to open a file that doesn't exist")
        .raw_error(),
        wasi::ERRNO_NOENT
    );
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
    unsafe { test_path_open_missing(dir_fd) }
}
