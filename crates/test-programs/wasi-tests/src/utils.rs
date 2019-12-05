use crate::wasi_wrappers::*;
use more_asserts::assert_gt;
use wasi_old::wasi_unstable;

pub unsafe fn create_dir(dir_fd: wasi_unstable::Fd, dir_name: &str) {
    assert!(
        wasi_path_create_directory(dir_fd, dir_name).is_ok(),
        "creating a directory"
    );
}

pub unsafe fn cleanup_dir(dir_fd: wasi_unstable::Fd, dir_name: &str) {
    assert!(
        wasi_path_remove_directory(dir_fd, dir_name).is_ok(),
        "remove_directory on an empty directory should succeed"
    );
}

/// Create an empty file with the given name.
pub unsafe fn create_file(dir_fd: wasi_unstable::Fd, file_name: &str) {
    let mut file_fd = wasi_unstable::Fd::max_value() - 1;
    let status = wasi_path_open(
        dir_fd,
        0,
        file_name,
        wasi_unstable::O_CREAT,
        0,
        0,
        0,
        &mut file_fd,
    );
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "creating a file"
    );
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi_unstable::Fd,
        "file descriptor range check",
    );
    close_fd(file_fd);
}

pub unsafe fn cleanup_file(dir_fd: wasi_unstable::Fd, file_name: &str) {
    assert!(
        wasi_path_unlink_file(dir_fd, file_name).is_ok(),
        "unlink_file on a symlink should succeed"
    );
}

pub unsafe fn close_fd(fd: wasi_unstable::Fd) {
    assert!(wasi_unstable::fd_close(fd).is_ok(), "closing a file");
}
