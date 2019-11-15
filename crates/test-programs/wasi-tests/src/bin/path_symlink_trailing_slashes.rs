use std::{env, process};
use wasi::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_dir, cleanup_file, create_dir, create_file};
use wasi_tests::wasi_wrappers::wasi_path_symlink;

unsafe fn test_path_symlink_trailing_slashes(dir_fd: wasi_unstable::Fd) {
    // Link destination shouldn't end with a slash.
    assert_eq!(
        wasi_path_symlink("source", dir_fd, "target/"),
        Err(wasi_unstable::ENOENT),
        "link destination ending with a slash"
    );

    // Without the trailing slash, this should succeed.
    assert_eq!(
        wasi_path_symlink("source", dir_fd, "target"),
        Ok(()),
        "link destination ending with a slash"
    );
    cleanup_file(dir_fd, "target");

    // Link destination already exists, target has trailing slash.
    create_dir(dir_fd, "target");
    assert_eq!(
        wasi_path_symlink("source", dir_fd, "target/"),
        Err(wasi_unstable::EEXIST),
        "link destination already exists"
    );
    cleanup_dir(dir_fd, "target");

    // Link destination already exists, target has no trailing slash.
    create_dir(dir_fd, "target");
    assert_eq!(
        wasi_path_symlink("source", dir_fd, "target"),
        Err(wasi_unstable::EEXIST),
        "link destination already exists"
    );
    cleanup_dir(dir_fd, "target");

    // Link destination already exists, target has trailing slash.
    create_file(dir_fd, "target");
    assert_eq!(
        wasi_path_symlink("source", dir_fd, "target/"),
        Err(wasi_unstable::EEXIST),
        "link destination already exists"
    );
    cleanup_file(dir_fd, "target");

    // Link destination already exists, target has no trailing slash.
    create_file(dir_fd, "target");
    assert_eq!(
        wasi_path_symlink("source", dir_fd, "target"),
        Err(wasi_unstable::EEXIST),
        "link destination already exists"
    );
    cleanup_file(dir_fd, "target");
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
    unsafe { test_path_symlink_trailing_slashes(dir_fd) }
}
