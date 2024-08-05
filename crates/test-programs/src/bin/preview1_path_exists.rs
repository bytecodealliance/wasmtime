use std::{env, process};
use test_programs::preview1::{create_file, open_scratch_directory};

unsafe fn test_path_exists(dir_fd: wasi::Fd) {
    // Create a temporary directory
    wasi::path_create_directory(dir_fd, "subdir").expect("create directory");

    // Check directory exists:
    let file_stat = wasi::path_filestat_get(dir_fd, 0, "subdir").expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_DIRECTORY);

    // Should still exist with symlink follow flag:
    let file_stat = wasi::path_filestat_get(dir_fd, wasi::LOOKUPFLAGS_SYMLINK_FOLLOW, "subdir")
        .expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_DIRECTORY);

    // Create a file:
    create_file(dir_fd, "subdir/file");
    // Check directory exists:
    let file_stat = wasi::path_filestat_get(dir_fd, 0, "subdir/file").expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_REGULAR_FILE);

    // Should still exist with symlink follow flag:
    let file_stat =
        wasi::path_filestat_get(dir_fd, wasi::LOOKUPFLAGS_SYMLINK_FOLLOW, "subdir/file")
            .expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_REGULAR_FILE);

    // Create a symlink to a file:
    wasi::path_symlink("subdir/file", dir_fd, "link1").expect("create symlink");
    // Check symlink exists:
    let file_stat = wasi::path_filestat_get(dir_fd, 0, "link1").expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_SYMBOLIC_LINK);

    // Should still exist with symlink follow flag, pointing to regular file
    let file_stat = wasi::path_filestat_get(dir_fd, wasi::LOOKUPFLAGS_SYMLINK_FOLLOW, "link1")
        .expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_REGULAR_FILE);

    // Create a symlink to a dir:
    wasi::path_symlink("subdir", dir_fd, "link2").expect("create symlink");
    // Check symlink exists:
    let file_stat = wasi::path_filestat_get(dir_fd, 0, "link2").expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_SYMBOLIC_LINK);

    // Should still exist with symlink follow flag, pointing to directory
    let file_stat = wasi::path_filestat_get(dir_fd, wasi::LOOKUPFLAGS_SYMLINK_FOLLOW, "link2")
        .expect("reading file stats");
    assert_eq!(file_stat.filetype, wasi::FILETYPE_DIRECTORY);

    wasi::path_unlink_file(dir_fd, "link1").expect("clean up");
    wasi::path_unlink_file(dir_fd, "link2").expect("clean up");
    wasi::path_unlink_file(dir_fd, "subdir/file").expect("clean up");
    wasi::path_remove_directory(dir_fd, "subdir").expect("clean up");
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
    unsafe { test_path_exists(dir_fd) }
}
