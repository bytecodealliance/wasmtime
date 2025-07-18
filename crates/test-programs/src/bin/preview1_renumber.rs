use std::{env, process};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_renumber(dir_fd: wasi::Fd) {
    let pre_fd: wasi::Fd = (libc::STDERR_FILENO + 1) as wasi::Fd;

    assert!(dir_fd > pre_fd, "dir_fd number");

    // Create a file in the scratch directory.
    let fd_from = wasi::path_open(
        dir_fd,
        0,
        "file1",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_from > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Get fd_from fdstat attributes
    let fdstat_from =
        wasi::fd_fdstat_get(fd_from).expect("calling fd_fdstat on the open file descriptor");

    // Create another file in the scratch directory.
    let fd_to = wasi::path_open(
        dir_fd,
        0,
        "file2",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_to > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    // Renumber fd of file1 into fd of file2
    wasi::fd_renumber(fd_from, fd_to).expect("renumbering two descriptors");

    // Ensure that fd_from is closed
    assert_errno!(
        wasi::fd_close(fd_from).expect_err("closing already closed file descriptor"),
        wasi::ERRNO_BADF
    );

    // Ensure that fd_to is still open.
    let fdstat_to =
        wasi::fd_fdstat_get(fd_to).expect("calling fd_fdstat on the open file descriptor");
    assert_eq!(
        fdstat_from.fs_filetype, fdstat_to.fs_filetype,
        "expected fd_to have the same fdstat as fd_from"
    );
    assert_eq!(
        fdstat_from.fs_flags, fdstat_to.fs_flags,
        "expected fd_to have the same fdstat as fd_from"
    );
    assert_eq!(
        fdstat_from.fs_rights_base, fdstat_to.fs_rights_base,
        "expected fd_to have the same fdstat as fd_from"
    );
    assert_eq!(
        fdstat_from.fs_rights_inheriting, fdstat_to.fs_rights_inheriting,
        "expected fd_to have the same fdstat as fd_from"
    );

    wasi::fd_close(fd_to).expect("closing a file");

    wasi::fd_renumber(0, 0).expect("renumbering 0 to 0");
    let fd_file3 = wasi::path_open(
        dir_fd,
        0,
        "file3",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_file3 > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );

    wasi::fd_renumber(fd_file3, 127).expect("renumbering FD to 127");
    match wasi::fd_renumber(127, u32::MAX) {
        Err(wasi::ERRNO_NOMEM) => {
            // The preview1 adapter cannot handle more than 128 descriptors
            eprintln!("fd_renumber({fd_file3}, {}) returned NOMEM", u32::MAX)
        }
        res => res.expect("renumbering FD to `u32::MAX`"),
    }

    let fd_file4 = wasi::path_open(
        dir_fd,
        0,
        "file4",
        wasi::OFLAGS_CREAT,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_file4 > libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
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
    unsafe { test_renumber(dir_fd) }
}
