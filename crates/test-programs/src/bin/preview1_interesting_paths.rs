use std::{env, process};
use test_programs::preview1::{assert_errno, create_file, open_scratch_directory};

unsafe fn test_interesting_paths(dir_fd: wasip1::Fd, arg: &str) {
    // Create a directory in the scratch directory.
    wasip1::path_create_directory(dir_fd, "dir").expect("creating dir");

    // Create a directory in the directory we just created.
    wasip1::path_create_directory(dir_fd, "dir/nested").expect("creating a nested dir");

    // Create a file in the nested directory.
    create_file(dir_fd, "dir/nested/file");

    // Now open it with an absolute path.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "/dir/nested/file", 0, 0, 0, 0)
            .expect_err("opening a file with an absolute path"),
        wasip1::ERRNO_PERM
    );

    // Now open it with a path containing "..".
    let mut file_fd = wasip1::path_open(
        dir_fd,
        0,
        "dir/.//nested/../../dir/nested/../nested///./file",
        0,
        0,
        0,
        0,
    )
    .expect("opening a file with \"..\" in the path");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    wasip1::fd_close(file_fd).expect("closing a file");

    // Now open it with a trailing NUL. Windows will allow this.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "dir/nested/file\0", 0, 0, 0, 0)
            .expect_err("opening a file with a trailing NUL"),
        unix => wasip1::ERRNO_INVAL,
        windows => wasip1::ERRNO_NOENT
    );

    // Now open it with a trailing slash.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "dir/nested/file/", 0, 0, 0, 0)
            .expect_err("opening a file with a trailing slash should fail"),
        wasip1::ERRNO_NOTDIR,
        wasip1::ERRNO_NOENT
    );

    // Now open it with trailing slashes.
    assert_errno!(
        wasip1::path_open(dir_fd, 0, "dir/nested/file///", 0, 0, 0, 0)
            .expect_err("opening a file with trailing slashes should fail"),
        wasip1::ERRNO_NOTDIR,
        wasip1::ERRNO_NOENT
    );

    // Now open the directory with a trailing slash.
    file_fd = wasip1::path_open(dir_fd, 0, "dir/nested/", 0, 0, 0, 0)
        .expect("opening a directory with a trailing slash");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    wasip1::fd_close(file_fd).expect("closing a file");

    // Now open the directory with trailing slashes.
    file_fd = wasip1::path_open(dir_fd, 0, "dir/nested///", 0, 0, 0, 0)
        .expect("opening a directory with trailing slashes");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
    wasip1::fd_close(file_fd).expect("closing a file");

    // Now open it with a path containing too many ".."s.
    let bad_path = format!("dir/nested/../../../{arg}/dir/nested/file");
    assert_errno!(
        wasip1::path_open(dir_fd, 0, &bad_path, 0, 0, 0, 0)
            .expect_err("opening a file with too many \"..\"s in the path should fail"),
        wasip1::ERRNO_PERM
    );
    wasip1::path_unlink_file(dir_fd, "dir/nested/file")
        .expect("unlink_file on a symlink should succeed");
    wasip1::path_remove_directory(dir_fd, "dir/nested")
        .expect("remove_directory on a directory should succeed");
    wasip1::path_remove_directory(dir_fd, "dir")
        .expect("remove_directory on a directory should succeed");
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
    unsafe { test_interesting_paths(dir_fd, &arg) }
}
