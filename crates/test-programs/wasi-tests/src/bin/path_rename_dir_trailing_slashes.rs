use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_path_rename_trailing_slashes(dir_fd: wasi::Fd) {
    // Test renaming a directory with a trailing slash in the name.
    wasi::path_create_directory(dir_fd, "source").expect("creating a directory");
    wasi::path_rename(dir_fd, "source/", dir_fd, "target")
        .expect("renaming a directory with a trailing slash in the source name");
    wasi::path_rename(dir_fd, "target", dir_fd, "source/")
        .expect("renaming a directory with a trailing slash in the destination name");
    wasi::path_rename(dir_fd, "source/", dir_fd, "target/")
        .expect("renaming a directory with a trailing slash in the source and destination names");
    wasi::path_rename(dir_fd, "target", dir_fd, "source")
        .expect("renaming a directory with no trailing slashes at all should work");
    wasi::path_remove_directory(dir_fd, "source").expect("removing the directory");
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
