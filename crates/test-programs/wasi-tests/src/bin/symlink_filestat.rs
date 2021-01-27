use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_path_filestat(dir_fd: wasi::Fd) {
    let mut fdstat = wasi::fd_fdstat_get(dir_fd).expect("fd_fdstat_get");
    assert_ne!(
        fdstat.fs_rights_base & wasi::RIGHTS_PATH_FILESTAT_GET,
        0,
        "the scratch directory should have RIGHT_PATH_FILESTAT_GET as base right",
    );

    // Create a file in the scratch directory.
    let file_fd = wasi::path_open(
        dir_fd,
        0,
        "file",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE | wasi::RIGHTS_PATH_FILESTAT_GET,
        0,
        0,
    )
    .expect("opening a file");
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Check file size
    let file_stat = wasi::path_filestat_get(dir_fd, 0, "file").expect("reading file stats");
    assert_eq!(file_stat.size, 0, "file size should be 0");

    // Create a symlink
    wasi::path_symlink("file", dir_fd, "symlink").expect("creating symlink to a file");

    // Check path_filestat_set_times on the symlink itself
    let sym_stat = wasi::path_filestat_get(dir_fd, 0, "symlink").expect("reading symlink stats");

    // Modify mtim of symlink
    let sym_new_mtim = sym_stat.mtim - 200;
    wasi::path_filestat_set_times(dir_fd, 0, "symlink", 0, sym_new_mtim, wasi::FSTFLAGS_MTIM)
        .expect("path_filestat_set_times should succeed on symlink");

    // Check that mtim motification worked
    let modified_sym_stat = wasi::path_filestat_get(dir_fd, 0, "symlink")
        .expect("reading file stats after path_filestat_set_times");
    assert_eq!(modified_sym_stat.mtim, sym_new_mtim, "mtim should change");

    // Now, dereference the symlink
    let deref_sym_stat =
        wasi::path_filestat_get(dir_fd, wasi::LOOKUPFLAGS_SYMLINK_FOLLOW, "symlink")
            .expect("reading file stats on the dereferenced symlink");
    assert_eq!(
        deref_sym_stat.mtim, file_stat.mtim,
        "symlink mtim should be equal to pointee's when dereferenced"
    );

    // Finally, change stat of the original file by dereferencing the symlink
    wasi::path_filestat_set_times(
        dir_fd,
        wasi::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        0,
        sym_stat.mtim,
        wasi::FSTFLAGS_MTIM,
    )
    .expect("path_filestat_set_times should succeed on setting stat on original file");

    let new_file_stat = wasi::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after path_filestat_set_times");
    assert_eq!(new_file_stat.mtim, sym_stat.mtim, "mtim should change");

    wasi::fd_close(file_fd).expect("closing a file");
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");
    wasi::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_path_filestat(dir_fd) }
}
