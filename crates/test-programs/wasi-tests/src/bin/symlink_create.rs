use libc;
use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn create_symlink_to_file(dir_fd: wasi::Fd) {
    // Create a directory for the symlink to point to.
    let target_fd =
        wasi::path_open(dir_fd, 0, "target", wasi::OFLAGS_CREAT, 0, 0, 0).expect("creating a file");
    wasi::fd_close(target_fd).expect("closing file");

    // Create a symlink.
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory without O_NOFOLLOW.
    let target_file_via_symlink = wasi::path_open(
        dir_fd,
        wasi::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        0,
        0,
        0,
        0,
    )
    .expect("opening a symlink as a directory");
    assert_gt!(
        target_file_via_symlink,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    wasi::fd_close(target_file_via_symlink).expect("close the symlink file");

    // Replace the target directory with a file.
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing the symlink");
    wasi::path_unlink_file(dir_fd, "target").expect("removing the target file");
}

unsafe fn create_symlink_to_directory(dir_fd: wasi::Fd) {
    // Create a directory for the symlink to point to.
    wasi::path_create_directory(dir_fd, "target").expect("creating a dir");

    // Create a symlink.
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory without O_NOFOLLOW.
    let target_dir_via_symlink = wasi::path_open(
        dir_fd,
        wasi::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        wasi::OFLAGS_DIRECTORY,
        0,
        0,
        0,
    )
    .expect("opening a symlink as a directory");
    assert_gt!(
        target_dir_via_symlink,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    wasi::fd_close(target_dir_via_symlink).expect("closing a file");

    // Replace the target directory with a file.
    wasi::path_unlink_file(dir_fd, "symlink").expect("remove symlink to directory");
    wasi::path_remove_directory(dir_fd, "target")
        .expect("remove_directory on a directory should succeed");
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
    unsafe {
        create_symlink_to_file(dir_fd);
        create_symlink_to_directory(dir_fd);
    }
}
