use libc;
use more_asserts::assert_gt;
use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{close_fd, create_dir, create_file};
use wasi_tests::wasi_wrappers::{
    wasi_path_open, wasi_path_remove_directory, wasi_path_unlink_file,
};

unsafe fn test_interesting_paths(dir_fd: wasi_unstable::Fd, arg: &str) {
    // Create a directory in the scratch directory.
    create_dir(dir_fd, "dir");

    // Create a directory in the directory we just created.
    create_dir(dir_fd, "dir/nested");

    // Create a file in the nested directory.
    create_file(dir_fd, "dir/nested/file");

    // Now open it with an absolute path.
    let mut file_fd: wasi_unstable::Fd = wasi_unstable::Fd::max_value() - 1;
    let mut status = wasi_path_open(dir_fd, 0, "/dir/nested/file", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOTCAPABLE,
        "opening a file with an absolute path"
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Now open it with a path containing "..".
    status = wasi_path_open(
        dir_fd,
        0,
        "dir/.//nested/../../dir/nested/../nested///./file",
        0,
        0,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a file with \"..\" in the path"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    close_fd(file_fd);

    // Now open it with a trailing NUL.
    status = wasi_path_open(dir_fd, 0, "dir/nested/file\0", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_EILSEQ,
        "opening a file with a trailing NUL"
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Now open it with a trailing slash.
    status = wasi_path_open(dir_fd, 0, "dir/nested/file/", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOTDIR,
        "opening a file with a trailing slash"
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Now open it with trailing slashes.
    status = wasi_path_open(dir_fd, 0, "dir/nested/file///", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOTDIR,
        "opening a file with trailing slashes"
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );

    // Now open the directory with a trailing slash.
    status = wasi_path_open(dir_fd, 0, "dir/nested/", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a directory with a trailing slash"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    close_fd(file_fd);

    // Now open the directory with trailing slashes.
    status = wasi_path_open(dir_fd, 0, "dir/nested///", 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "opening a directory with trailing slashes"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    close_fd(file_fd);

    // Now open it with a path containing too many ".."s.
    let bad_path = format!("dir/nested/../../../{}/dir/nested/file", arg);
    status = wasi_path_open(dir_fd, 0, &bad_path, 0, 0, 0, 0, &mut file_fd);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ENOTCAPABLE,
        "opening a file with too many \"..\"s in the path"
    );
    assert_eq!(
        file_fd,
        wasi_unstable::Fd::max_value(),
        "failed open should set the file descriptor to -1",
    );
    assert!(
        wasi_path_unlink_file(dir_fd, "dir/nested/file").is_ok(),
        "unlink_file on a symlink should succeed"
    );
    assert!(
        wasi_path_remove_directory(dir_fd, "dir/nested").is_ok(),
        "remove_directory on a directory should succeed"
    );
    assert!(
        wasi_path_remove_directory(dir_fd, "dir").is_ok(),
        "remove_directory on a directory should succeed"
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
    unsafe { test_interesting_paths(dir_fd, &arg) }
}
