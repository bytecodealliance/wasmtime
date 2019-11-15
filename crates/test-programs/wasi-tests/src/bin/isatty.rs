use more_asserts::assert_gt;
use std::{env, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd};
use wasi_tests::wasi_wrappers::wasi_path_open;

unsafe fn test_isatty(dir_fd: wasi_unstable::Fd) {
    // Create a file in the scratch directory and test if it's a tty.
    let mut file_fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT,
        0,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    assert_eq!(
        libc::isatty(file_fd as std::os::raw::c_int),
        0,
        "file is a tty"
    );
    close_fd(file_fd);

    cleanup_file(dir_fd, "file");
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
    unsafe { test_isatty(dir_fd) }
}
