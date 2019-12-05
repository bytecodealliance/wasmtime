use more_asserts::assert_gt;
use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_file, close_fd};
use wasi_tests::wasi_wrappers::{wasi_fd_filestat_get, wasi_path_open};

unsafe fn test_file_allocate(dir_fd: wasi_unstable::Fd) {
    // Create a file in the scratch directory.
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(
        dir_fd,
        0,
        "file",
        wasi_unstable::O_CREAT,
        wasi_unstable::RIGHT_FD_READ | wasi_unstable::RIGHT_FD_WRITE,
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

    // Check file size
    let mut stat = wasi_unstable::FileStat {
        st_dev: 0,
        st_ino: 0,
        st_filetype: 0,
        st_nlink: 0,
        st_size: 0,
        st_atim: 0,
        st_mtim: 0,
        st_ctim: 0,
    };
    let status = wasi_fd_filestat_get(file_fd, &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats"
    );
    assert_eq!(stat.st_size, 0, "file size should be 0");

    // Allocate some size
    assert!(
        wasi_unstable::fd_allocate(file_fd, 0, 100).is_ok(),
        "allocating size"
    );

    let status = wasi_fd_filestat_get(file_fd, &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after initial allocation"
    );
    assert_eq!(stat.st_size, 100, "file size should be 100");

    // Allocate should not modify if less than current size
    assert!(
        wasi_unstable::fd_allocate(file_fd, 10, 10).is_ok(),
        "allocating size less than current size"
    );

    let status = wasi_fd_filestat_get(file_fd, &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after additional allocation was not required"
    );
    assert_eq!(
        stat.st_size, 100,
        "file size should remain unchanged at 100"
    );

    // Allocate should modify if offset+len > current_len
    assert!(
        wasi_unstable::fd_allocate(file_fd, 90, 20).is_ok(),
        "allocating size larger than current size"
    );

    let status = wasi_fd_filestat_get(file_fd, &mut stat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "reading file stats after additional allocation was required"
    );
    assert_eq!(
        stat.st_size, 110,
        "file size should increase from 100 to 110"
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
    unsafe { test_file_allocate(dir_fd) }
}
