use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn create_symlink_to_file(dir_fd: wasip1::Fd) {
    // Create a directory for the symlink to point to.
    let target_fd = wasip1::path_open(dir_fd, 0, "target", wasip1::OFLAGS_CREAT, 0, 0, 0)
        .expect("creating a file");
    wasip1::fd_close(target_fd).expect("closing file");

    // Create a symlink.
    wasip1::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory without O_NOFOLLOW.
    let target_file_via_symlink = wasip1::path_open(
        dir_fd,
        wasip1::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        0,
        0,
        0,
        0,
    )
    .expect("opening a symlink as a directory");
    assert!(
        target_file_via_symlink > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    wasip1::fd_close(target_file_via_symlink).expect("close the symlink file");

    // Replace the target directory with a file.
    wasip1::path_unlink_file(dir_fd, "symlink").expect("removing the symlink");
    wasip1::path_unlink_file(dir_fd, "target").expect("removing the target file");
}

unsafe fn create_symlink_to_directory(dir_fd: wasip1::Fd) {
    // Create a directory for the symlink to point to.
    wasip1::path_create_directory(dir_fd, "target").expect("creating a dir");

    // Create a symlink.
    wasip1::path_symlink("target", dir_fd, "symlink").expect("creating a symlink");

    // Try to open it as a directory without O_NOFOLLOW.
    let target_dir_via_symlink = wasip1::path_open(
        dir_fd,
        wasip1::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        wasip1::OFLAGS_DIRECTORY,
        0,
        0,
        0,
    )
    .expect("opening a symlink as a directory");
    assert!(
        target_dir_via_symlink > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    wasip1::fd_close(target_dir_via_symlink).expect("closing a file");

    // Replace the target directory with a file.
    wasip1::path_unlink_file(dir_fd, "symlink").expect("remove symlink to directory");
    wasip1::path_remove_directory(dir_fd, "target")
        .expect("remove_directory on a directory should succeed");
}

unsafe fn create_symlink_to_root(dir_fd: wasip1::Fd) {
    // Create a symlink.
    wasip1::path_symlink("/", dir_fd, "symlink")
        .expect_err("creating a symlink to an absolute path");
}

fn main() {
    let mut args = env::args();
    let prog = args.next().unwrap();
    let arg = if let Some(arg) = args.next() {
        arg
    } else {
        eprintln!("usage: {prog} <scratch directory>");
        process::exit(1);
    };

    // Open scratch directory
    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1)
        }
    };

    // Run the tests.
    unsafe {
        create_symlink_to_file(dir_fd);
        create_symlink_to_directory(dir_fd);
        create_symlink_to_root(dir_fd);
    }
}
