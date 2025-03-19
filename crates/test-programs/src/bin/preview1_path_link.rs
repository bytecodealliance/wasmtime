use std::{env, process};
use test_programs::preview1::{assert_errno, config, create_file, open_scratch_directory};

// These are all macro-rules so the panic line number shows us where
// things went wrong.

macro_rules! filestats_assert_eq {
    ($left:ident, $right:ident) => {
        assert_eq!($left.dev, $right.dev, "dev should be equal");
        assert_eq!($left.ino, $right.ino, "ino should be equal");
        assert_eq!($left.atim, $right.atim, "atim should be equal");
        assert_eq!($left.ctim, $right.ctim, "ctim should be equal");
        assert_eq!($left.mtim, $right.mtim, "mtim should be equal");
        assert_eq!($left.size, $right.size, "size should be equal");
        assert_eq!($left.nlink, $right.nlink, "nlink should be equal");
        assert_eq!($left.filetype, $right.filetype, "filetype should be equal");
    };
}

macro_rules! fdstats_assert_eq {
    ($left:ident, $right:ident) => {
        assert_eq!($left.fs_flags, $right.fs_flags, "fs_flags should be equal");
        assert_eq!(
            $left.fs_filetype, $right.fs_filetype,
            "fs_filetype should be equal"
        );
        assert_eq!(
            $left.fs_rights_base, $right.fs_rights_base,
            "fs_rights_base should be equal"
        );
        assert_eq!(
            $left.fs_rights_inheriting, $right.fs_rights_inheriting,
            "fs_rights_inheriting should be equal"
        );
    };
}

macro_rules! check_rights {
    ($orig_fd:ident, $link_fd:ident) => {
        let orig_filestat =
            wasip1::fd_filestat_get($orig_fd).expect("reading filestat of the source");
        let link_filestat =
            wasip1::fd_filestat_get($link_fd).expect("reading filestat of the link");
        filestats_assert_eq!(orig_filestat, link_filestat);

        // Compare Fdstats
        let orig_fdstat = wasip1::fd_fdstat_get($orig_fd).expect("reading fdstat of the source");
        let link_fdstat = wasip1::fd_fdstat_get($link_fd).expect("reading fdstat of the link");
        fdstats_assert_eq!(orig_fdstat, link_fdstat);
    };
}
// Extra calls of fd_close are needed for Windows, which will not remove
// the directory until all handles are closed.
unsafe fn test_path_link(dir_fd: wasip1::Fd) {
    // Create a file
    let create_fd =
        wasip1::path_open(dir_fd, 0, "file", wasip1::OFLAGS_CREAT, 0, 0, 0).expect("create file");
    wasip1::fd_close(create_fd).unwrap();

    // Open a fresh descriptor to the file. We won't have a write right that was implied by OFLAGS_CREAT
    // above.
    let file_fd = wasip1::path_open(dir_fd, 0, "file", 0, 0, 0, 0).expect("open file");

    // Create a link in the same directory and compare rights
    wasip1::path_link(dir_fd, 0, "file", dir_fd, "link")
        .expect("creating a link in the same directory");

    let link_fd = wasip1::path_open(dir_fd, 0, "link", 0, 0, 0, 0).expect("open link");

    check_rights!(file_fd, link_fd);
    wasip1::fd_close(link_fd).expect("Closing link_fd"); // needed for Windows
    wasip1::path_unlink_file(dir_fd, "link").expect("removing a link");

    // Create a link in a different directory and compare rights
    wasip1::path_create_directory(dir_fd, "subdir").expect("creating a subdirectory");
    let subdir_fd = wasip1::path_open(dir_fd, 0, "subdir", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("open subdir directory");
    wasip1::path_link(dir_fd, 0, "file", subdir_fd, "link")
        .expect("creating a link in subdirectory");
    let link_fd = wasip1::path_open(subdir_fd, 0, "link", 0, 0, 0, 0).expect("open link in subdir");
    check_rights!(file_fd, link_fd);
    wasip1::fd_close(link_fd).expect("Closing link_fd"); // needed for Windows
    wasip1::path_unlink_file(subdir_fd, "link").expect("removing a link");
    wasip1::fd_close(subdir_fd).expect("Closing subdir_fd"); // needed for Windows
    wasip1::path_remove_directory(dir_fd, "subdir").expect("removing a subdirectory");

    // Create a link to a path that already exists
    create_file(dir_fd, "link");

    assert_errno!(
        wasip1::path_link(dir_fd, 0, "file", dir_fd, "link")
            .expect_err("creating a link to existing path should fail"),
        wasip1::ERRNO_EXIST
    );
    wasip1::path_unlink_file(dir_fd, "link").expect("removing a file");

    // Create a link to itself
    assert_errno!(
        wasip1::path_link(dir_fd, 0, "file", dir_fd, "file")
            .expect_err("creating a link to itself should fail"),
        wasip1::ERRNO_EXIST
    );

    // Create a link where target is a directory
    wasip1::path_create_directory(dir_fd, "link").expect("creating a dir");

    assert_errno!(
        wasip1::path_link(dir_fd, 0, "file", dir_fd, "link")
            .expect_err("creating a link where target is a directory should fail"),
        wasip1::ERRNO_EXIST
    );
    wasip1::path_remove_directory(dir_fd, "link").expect("removing a dir");

    // Create a link to a directory
    wasip1::path_create_directory(dir_fd, "subdir").expect("creating a subdirectory");
    let subdir_fd = wasip1::path_open(dir_fd, 0, "subdir", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("open new descriptor to subdir");

    assert_errno!(
        wasip1::path_link(dir_fd, 0, "subdir", dir_fd, "link")
            .expect_err("creating a link to a directory should fail"),
        wasip1::ERRNO_PERM,
        wasip1::ERRNO_ACCES
    );
    wasip1::fd_close(subdir_fd).expect("close subdir before deleting it");
    wasip1::path_remove_directory(dir_fd, "subdir").expect("removing a subdirectory");

    // Create a link to a file with trailing slash
    assert_errno!(
        wasip1::path_link(dir_fd, 0, "file", dir_fd, "link/")
            .expect_err("creating a link to a file with trailing slash should fail"),
        wasip1::ERRNO_NOENT
    );

    if config().support_dangling_filesystem() {
        // Create a link to a dangling symlink
        wasip1::path_symlink("target", dir_fd, "symlink").expect("creating a dangling symlink");

        // This should succeed, because we're not following symlinks
        wasip1::path_link(dir_fd, 0, "symlink", dir_fd, "link")
            .expect("creating a link to a dangling symlink should succeed");
        wasip1::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");
        wasip1::path_unlink_file(dir_fd, "link").expect("removing a hardlink");

        // Create a link to a symlink loop
        wasip1::path_symlink("symlink", dir_fd, "symlink").expect("creating a symlink loop");

        wasip1::path_link(dir_fd, 0, "symlink", dir_fd, "link")
            .expect("creating a link to a symlink loop should succeed");
        wasip1::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");
        wasip1::path_unlink_file(dir_fd, "link").expect("removing a hardlink");

        // Create a link where target is a dangling symlink
        wasip1::path_symlink("target", dir_fd, "symlink").expect("creating a dangling symlink");

        assert_errno!(
            wasip1::path_link(dir_fd, 0, "file", dir_fd, "symlink")
                .expect_err("creating a link where target is a dangling symlink"),
            wasip1::ERRNO_EXIST
        );
        wasip1::path_unlink_file(dir_fd, "symlink").expect("removing a symlink");

        // Create a link where target is a dangling symlink following symlinks
        wasip1::path_symlink("target", dir_fd, "symlink").expect("creating a dangling symlink");

        // Symlink following with path_link is rejected
        assert_errno!(
            wasip1::path_link(
                dir_fd,
                wasip1::LOOKUPFLAGS_SYMLINK_FOLLOW,
                "symlink",
                dir_fd,
                "link",
            )
            .expect_err("calling path_link with LOOKUPFLAGS_SYMLINK_FOLLOW should fail"),
            wasip1::ERRNO_INVAL
        );

        // Clean up.
        wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
    }
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
    unsafe { test_path_link(dir_fd) }
}
