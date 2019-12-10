use std::{env, process};
use wasi_old::wasi_unstable;
use wasi_tests::open_scratch_directory;
use wasi_tests::utils::{cleanup_dir, cleanup_file, create_dir, create_file};
use wasi_tests::wasi_wrappers::wasi_path_rename;

unsafe fn test_path_rename_trailing_slashes(dir_fd: wasi_unstable::Fd) {
    // Test renaming a file with a trailing slash in the name.
    create_file(dir_fd, "source");
    assert_eq!(
        wasi_path_rename(dir_fd, "source/", dir_fd, "target"),
        Err(wasi_unstable::ENOTDIR),
        "renaming a file with a trailing slash in the source name"
    );
    assert_eq!(
        wasi_path_rename(dir_fd, "source", dir_fd, "target/"),
        Err(wasi_unstable::ENOTDIR),
        "renaming a file with a trailing slash in the destination name"
    );
    assert_eq!(
        wasi_path_rename(dir_fd, "source/", dir_fd, "target/"),
        Err(wasi_unstable::ENOTDIR),
        "renaming a file with a trailing slash in the source and destination names"
    );
    cleanup_file(dir_fd, "source");

    // Test renaming a directory with a trailing slash in the name.
    create_dir(dir_fd, "source");
    assert_eq!(
        wasi_path_rename(dir_fd, "source/", dir_fd, "target"),
        Ok(()),
        "renaming a directory with a trailing slash in the source name"
    );
    assert_eq!(
        wasi_path_rename(dir_fd, "target", dir_fd, "source/"),
        Ok(()),
        "renaming a directory with a trailing slash in the destination name"
    );
    assert_eq!(
        wasi_path_rename(dir_fd, "source/", dir_fd, "target/"),
        Ok(()),
        "renaming a directory with a trailing slash in the source and destination names"
    );
    cleanup_dir(dir_fd, "target");
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
    unsafe { test_path_rename_trailing_slashes(dir_fd) }
}
