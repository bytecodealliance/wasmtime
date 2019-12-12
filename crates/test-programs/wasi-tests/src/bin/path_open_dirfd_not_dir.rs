use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::close_fd;
use wasi_tests::wasi_wrappers::wasi_path_open;

unsafe fn test_dirfd_not_dir(dir_fd: wasi_unstable::Fd) {
    // Open a file.
    let mut file_fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(
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

    // Now try to open a file underneath it as if it were a directory.
    let mut new_file_fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    status = wasi_path_open(
        file_fd,
        0,
        "foo",
        wasi_unstable::O_CREAT,
        0,
        0,
        0,
        &mut new_file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOTDIR,
        "non-directory base fd should get ENOTDIR"
    );
    close_fd(file_fd);
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
    unsafe { test_dirfd_not_dir(dir_fd) }
}
