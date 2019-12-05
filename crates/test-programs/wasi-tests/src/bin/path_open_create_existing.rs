use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd};
use wasi_tests::wasi_wrappers::wasi_path_open;

unsafe fn test_path_open_create_existing(dir_fd: wasi_unstable::Fd) {
    let mut fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT | wasi_unstable::O_EXCL,
        0,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "creating a file"
    );
    close_fd(fd);
    fd = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT | wasi_unstable::O_EXCL,
        0,
        0,
        0,
        &mut fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_EEXIST,
        "trying to create a file that already exists"
    );
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
    unsafe { test_path_open_create_existing(dir_fd) }
}
