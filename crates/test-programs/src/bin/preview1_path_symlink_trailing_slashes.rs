use std::{env, process};
use test_programs::preview1::{assert_errno, config, create_file, open_scratch_directory};

unsafe fn test_path_symlink_trailing_slashes(dir_fd: wasip1::Fd) {
    if config().support_dangling_filesystem() {
        // Dangling symlink: Link destination shouldn't end with a slash.
        assert_errno!(
            wasip1::path_symlink("source", dir_fd, "target/")
                .expect_err("link destination ending with a slash should fail"),
            wasip1::ERRNO_NOENT
        );

        // Dangling symlink: Without the trailing slash, this should succeed.
        wasip1::path_symlink("source", dir_fd, "target")
            .expect("link destination ending with a slash");
        wasip1::path_unlink_file(dir_fd, "target").expect("removing a file");
    }

    // Link destination already exists, target has trailing slash.
    wasip1::path_create_directory(dir_fd, "target").expect("creating a directory");
    assert_errno!(
        wasip1::path_symlink("source", dir_fd, "target/")
            .expect_err("link destination already exists"),
        unix => wasip1::ERRNO_EXIST,
        windows => wasip1::ERRNO_NOENT
    );
    wasip1::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Link destination already exists, target has no trailing slash.
    wasip1::path_create_directory(dir_fd, "target").expect("creating a directory");
    assert_errno!(
        wasip1::path_symlink("source", dir_fd, "target")
            .expect_err("link destination already exists"),
        unix => wasip1::ERRNO_EXIST,
        windows => wasip1::ERRNO_NOENT
    );
    wasip1::path_remove_directory(dir_fd, "target").expect("removing a directory");

    // Link destination already exists, target has trailing slash.
    create_file(dir_fd, "target");

    assert_errno!(
        wasip1::path_symlink("source", dir_fd, "target/")
            .expect_err("link destination already exists"),
        unix => wasip1::ERRNO_NOTDIR,
        windows => wasip1::ERRNO_NOENT
    );
    wasip1::path_unlink_file(dir_fd, "target").expect("removing a file");

    // Link destination already exists, target has no trailing slash.
    create_file(dir_fd, "target");

    assert_errno!(
        wasip1::path_symlink("source", dir_fd, "target")
            .expect_err("link destination already exists"),
        unix => wasip1::ERRNO_EXIST,
        windows => wasip1::ERRNO_NOENT
    );
    wasip1::path_unlink_file(dir_fd, "target").expect("removing a file");
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
    unsafe { test_path_symlink_trailing_slashes(dir_fd) }
}
