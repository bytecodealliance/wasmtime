#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, process, time::Duration};
use test_programs::preview1::{TestConfig, assert_fs_time_eq, open_scratch_directory};

unsafe fn test_path_filestat(dir_fd: wasip1::Fd) {
    let cfg = TestConfig::from_env();
    // Create a file in the scratch directory.
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        "file",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    // Check file size
    let file_stat = wasip1::path_filestat_get(dir_fd, 0, "file").expect("reading file stats");
    assert_eq!(file_stat.size, 0, "file size should be 0");

    // Create a symlink
    wasip1::path_symlink("file", dir_fd, "symlink").expect("creating symlink to a file");

    // Check path_filestat_set_times on the symlink itself
    let sym_stat = wasip1::path_filestat_get(dir_fd, 0, "symlink").expect("reading symlink stats");

    // Modify mtim of symlink
    let sym_new_mtim = Duration::from_nanos(sym_stat.mtim) - cfg.fs_time_precision() * 2;
    wasip1::path_filestat_set_times(
        dir_fd,
        0,
        "symlink",
        0,
        sym_new_mtim.as_nanos() as u64,
        wasip1::FSTFLAGS_MTIM,
    )
    .expect("path_filestat_set_times should succeed on symlink");

    // Check that symlink mtim motification worked
    let modified_sym_stat = wasip1::path_filestat_get(dir_fd, 0, "symlink")
        .expect("reading file stats after path_filestat_set_times");

    assert_fs_time_eq!(
        Duration::from_nanos(modified_sym_stat.mtim),
        sym_new_mtim,
        "symlink mtim should change"
    );

    // Check that pointee mtim is not modified
    let unmodified_file_stat = wasip1::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after path_filestat_set_times");

    assert_eq!(
        unmodified_file_stat.mtim, file_stat.mtim,
        "file mtim should not change"
    );

    // Now, dereference the symlink
    let deref_sym_stat =
        wasip1::path_filestat_get(dir_fd, wasip1::LOOKUPFLAGS_SYMLINK_FOLLOW, "symlink")
            .expect("reading file stats on the dereferenced symlink");
    assert_eq!(
        deref_sym_stat.mtim, file_stat.mtim,
        "symlink mtim should be equal to pointee's when dereferenced"
    );

    // Finally, change stat of the original file by dereferencing the symlink
    wasip1::path_filestat_set_times(
        dir_fd,
        wasip1::LOOKUPFLAGS_SYMLINK_FOLLOW,
        "symlink",
        0,
        sym_stat.mtim,
        wasip1::FSTFLAGS_MTIM,
    )
    .expect("path_filestat_set_times should succeed on setting stat on original file");

    let new_file_stat = wasip1::path_filestat_get(dir_fd, 0, "file")
        .expect("reading file stats after path_filestat_set_times");

    assert_fs_time_eq!(
        Duration::from_nanos(new_file_stat.mtim),
        Duration::from_nanos(sym_stat.mtim),
        "mtim should change"
    );

    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");
    wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_path_filestat(dir_fd) }
}
