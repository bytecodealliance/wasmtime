use more_asserts::assert_gt;
use std::{env, process};
use wasi_tests::{assert_errno, create_file, open_scratch_directory};

const TEST_RIGHTS: wasi::Rights = wasi::RIGHTS_FD_READ
    | wasi::RIGHTS_PATH_LINK_SOURCE
    | wasi::RIGHTS_PATH_LINK_TARGET
    | wasi::RIGHTS_FD_FILESTAT_GET
    | wasi::RIGHTS_PATH_OPEN
    | wasi::RIGHTS_PATH_UNLINK_FILE;

unsafe fn create_or_open(dir_fd: wasi::Fd, name: &str, flags: wasi::Oflags) -> wasi::Fd {
    let file_fd = wasi::path_open(dir_fd, 0, name, flags, TEST_RIGHTS, TEST_RIGHTS, 0)
        .unwrap_or_else(|_| panic!("opening '{}'", name));
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    file_fd
}

unsafe fn open_link(dir_fd: wasi::Fd, name: &str) -> wasi::Fd {
    let file_fd = wasi::path_open(dir_fd, 0, name, 0, TEST_RIGHTS, TEST_RIGHTS, 0)
        .unwrap_or_else(|_| panic!("opening a link '{}'", name));
    assert_gt!(
        file_fd,
        libc::STDERR_FILENO as wasi::Fd,
        "file descriptor range check",
    );
    file_fd
}

// This is temporary until `wasi` implements `Debug` and `PartialEq` for
// `wasi::Filestat`.
fn filestats_assert_eq(left: wasi::Filestat, right: wasi::Filestat) {
    assert_eq!(left.dev, right.dev, "dev should be equal");
    assert_eq!(left.ino, right.ino, "ino should be equal");
    assert_eq!(left.atim, right.atim, "atim should be equal");
    assert_eq!(left.ctim, right.ctim, "ctim should be equal");
    assert_eq!(left.mtim, right.mtim, "mtim should be equal");
    assert_eq!(left.size, right.size, "size should be equal");
    assert_eq!(left.nlink, right.nlink, "nlink should be equal");
    assert_eq!(left.filetype, right.filetype, "filetype should be equal");
}

// This is temporary until `wasi` implements `Debug` and `PartialEq` for
// `wasi::Fdstat`.
fn fdstats_assert_eq(left: wasi::Fdstat, right: wasi::Fdstat) {
    assert_eq!(left.fs_flags, right.fs_flags, "fs_flags should be equal");
    assert_eq!(
        left.fs_filetype, right.fs_filetype,
        "fs_filetype should be equal"
    );
    assert_eq!(
        left.fs_rights_base, right.fs_rights_base,
        "fs_rights_base should be equal"
    );
    assert_eq!(
        left.fs_rights_inheriting, right.fs_rights_inheriting,
        "fs_rights_inheriting should be equal"
    );
}

unsafe fn check_rights(orig_fd: wasi::Fd, link_fd: wasi::Fd) {
    // Compare Filestats
    let orig_filestat = wasi::fd_filestat_get(orig_fd).expect("reading filestat of the source");
    let link_filestat = wasi::fd_filestat_get(link_fd).expect("reading filestat of the link");
    filestats_assert_eq(orig_filestat, link_filestat);

    // Compare Fdstats
    let orig_fdstat = wasi::fd_fdstat_get(orig_fd).expect("reading fdstat of the source");
    let link_fdstat = wasi::fd_fdstat_get(link_fd).expect("reading fdstat of the link");
    fdstats_assert_eq(orig_fdstat, link_fdstat);
}

// Extra calls of fd_close are needed for Windows, which will not remove
// the directory until all handles are closed.
unsafe fn test_path_link(dir_fd: wasi::Fd) {
    // Create a file
    let file_fd = create_or_open(dir_fd, "file", wasi::OFLAGS_CREAT);

    // Create a link in the same directory and compare rights
    wasi::path_link(dir_fd, 0, "file", dir_fd, "link")
        .expect("creating a link in the same directory");
    let link_fd = open_link(dir_fd, "link");
    check_rights(file_fd, link_fd);
    wasi::fd_close(link_fd).expect("Closing link_fd"); // needed for Windows
    wasi::path_unlink_file(dir_fd, "link").expect("removing a link");

    // Create a link in a different directory and compare rights
    wasi::path_create_directory(dir_fd, "subdir").expect("creating a subdirectory");
    let subdir_fd = create_or_open(dir_fd, "subdir", wasi::OFLAGS_DIRECTORY);
    wasi::path_link(dir_fd, 0, "file", subdir_fd, "link").expect("creating a link in subdirectory");
    let link_fd = open_link(subdir_fd, "link");
    check_rights(file_fd, link_fd);
    wasi::path_unlink_file(subdir_fd, "link").expect("removing a link");
    wasi::fd_close(subdir_fd).expect("Closing subdir_fd"); // needed for Windows
    wasi::fd_close(link_fd).expect("Closing link_fd"); // needed for Windows
    wasi::path_remove_directory(dir_fd, "subdir").expect("removing a subdirectory");

    // Create a link to a path that already exists
    create_file(dir_fd, "link");

    assert_errno!(
        wasi::path_link(dir_fd, 0, "file", dir_fd, "link")
            .expect_err("creating a link to existing path should fail")
            .raw_error(),
        wasi::ERRNO_EXIST
    );
    wasi::path_unlink_file(dir_fd, "link").expect("removing a file");

    // Create a link to itself
    assert_errno!(
        wasi::path_link(dir_fd, 0, "file", dir_fd, "file")
            .expect_err("creating a link to itself should fail")
            .raw_error(),
        wasi::ERRNO_EXIST
    );

    // Create a link where target is a directory
    wasi::path_create_directory(dir_fd, "link").expect("creating a dir");

    assert_errno!(
        wasi::path_link(dir_fd, 0, "file", dir_fd, "link")
            .expect_err("creating a link where target is a directory should fail")
            .raw_error(),
        wasi::ERRNO_EXIST
    );
    wasi::path_remove_directory(dir_fd, "link").expect("removing a dir");

    // Create a link to a directory
    wasi::path_create_directory(dir_fd, "subdir").expect("creating a subdirectory");
    let subdir_fd = create_or_open(dir_fd, "subdir", wasi::OFLAGS_DIRECTORY);

    assert_errno!(
        wasi::path_link(dir_fd, 0, "subdir", dir_fd, "link")
            .expect_err("creating a link to a directory should fail")
            .raw_error(),
        wasi::ERRNO_PERM,
        wasi::ERRNO_ACCES
    );
    wasi::fd_close(subdir_fd).expect("close subdir before deleting it");
    wasi::path_remove_directory(dir_fd, "subdir").expect("removing a subdirectory");

    // Create a link to a file with trailing slash
    assert_errno!(
        wasi::path_link(dir_fd, 0, "file", dir_fd, "link/")
            .expect_err("creating a link to a file with trailing slash should fail")
            .raw_error(),
        wasi::ERRNO_NOENT
    );

    // XXX windows doesnt support dangling symlinks - rest of file
    // Create a link to a dangling symlink
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a dangling symlink");

    // This should succeed, because we're not following symlinks
    wasi::path_link(dir_fd, 0, "symlink", dir_fd, "link")
        .expect("creating a link to a dangling symlink should succeed");
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");
    wasi::path_unlink_file(dir_fd, "link").expect("removing a hardlink");

    // Create a link to a symlink loop
    wasi::path_symlink("symlink", dir_fd, "symlink").expect("creating a symlink loop");

    wasi::path_link(dir_fd, 0, "symlink", dir_fd, "link")
        .expect("creating a link to a symlink loop should succeed");
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");
    wasi::path_unlink_file(dir_fd, "link").expect("removing a hardlink");

    // Create a link where target is a dangling symlink
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a dangling symlink");

    assert_errno!(
        wasi::path_link(dir_fd, 0, "file", dir_fd, "symlink")
            .expect_err("creating a link where target is a dangling symlink")
            .raw_error(),
        wasi::ERRNO_EXIST
    );
    wasi::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");

    // Create a link where target is a dangling symlink following symlinks
    wasi::path_symlink("target", dir_fd, "symlink").expect("creating a dangling symlink");

    // Symlink following with path_link is rejected
    assert_errno!(
        wasi::path_link(
            dir_fd,
            wasi::LOOKUPFLAGS_SYMLINK_FOLLOW,
            "symlink",
            dir_fd,
            "link",
        )
        .expect_err("calling path_link with LOOKUPFLAGS_SYMLINK_FOLLOW should fail")
        .raw_error(),
        wasi::ERRNO_INVAL
    );

    // Clean up.
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
    unsafe { test_path_link(dir_fd) }
}
