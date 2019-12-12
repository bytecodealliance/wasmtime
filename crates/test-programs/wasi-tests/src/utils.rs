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

// Returns: (rights_base, rights_inheriting)
pub unsafe fn fd_get_rights(fd: wasi_unstable::Fd) -> (wasi_unstable::Rights, wasi_unstable::Rights) {
    let mut fdstat: wasi_unstable::FdStat = std::mem::zeroed();
    let status = wasi_fd_fdstat_get(fd, &mut fdstat);
    assert_eq!(
        status,
        wasi_unstable::raw::__WASI_ESUCCESS,
        "calling fd_fdstat_get"
    );

    (fdstat.fs_rights_base, fdstat.fs_rights_inheriting)
}

pub unsafe fn drop_rights(
    fd: wasi_unstable::Fd,
    drop_base: wasi_unstable::Rights,
    drop_inheriting: wasi_unstable::Rights,
) {
    let (current_base, current_inheriting) = fd_get_rights(fd);

    let new_base = current_base & !drop_base;
    let new_inheriting = current_inheriting & !drop_inheriting;

    assert!(
        wasi_unstable::fd_fdstat_set_rights(fd, new_base, new_inheriting).is_ok(),
        "dropping fd rights",
    );
}
