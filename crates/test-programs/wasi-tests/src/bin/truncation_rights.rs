use std::{env, process};
use wasi_tests::{assert_errno, create_file, open_scratch_directory};

unsafe fn test_truncation_rights(dir_fd: wasi::Fd) {
    // Create a file in the scratch directory.
    create_file(dir_fd, "file");

    // Get the rights for the scratch directory.
    let mut dir_fdstat =
        wasi::fd_fdstat_get(dir_fd).expect("calling fd_fdstat on the scratch directory");
    assert_eq!(
        dir_fdstat.fs_filetype,
        wasi::FILETYPE_DIRECTORY,
        "expected the scratch directory to be a directory",
    );
    assert_eq!(
        dir_fdstat.fs_flags, 0,
        "expected the scratch directory to have no special flags",
    );
    assert_eq!(
        dir_fdstat.fs_rights_base & wasi::RIGHTS_FD_FILESTAT_SET_SIZE,
        0,
        "directories shouldn't have the fd_filestat_set_size right",
    );

    // If we have the right to set sizes from paths, test that it works.
    if (dir_fdstat.fs_rights_base & wasi::RIGHTS_PATH_FILESTAT_SET_SIZE) == 0 {
        eprintln!("implementation doesn't support setting file sizes, skipping");
    } else {
        // Test that we can truncate the file.
        let mut file_fd = wasi::path_open(dir_fd, 0, "file", wasi::OFLAGS_TRUNC, 0, 0, 0)
            .expect("truncating a file");
        wasi::fd_close(file_fd).expect("closing a file");

        let mut rights_base: wasi::Rights = dir_fdstat.fs_rights_base;
        let mut rights_inheriting: wasi::Rights = dir_fdstat.fs_rights_inheriting;

        if (rights_inheriting & wasi::RIGHTS_FD_FILESTAT_SET_SIZE) == 0 {
            eprintln!("implementation doesn't support setting file sizes through file descriptors, skipping");
        } else {
            rights_inheriting &= !wasi::RIGHTS_FD_FILESTAT_SET_SIZE;
            wasi::fd_fdstat_set_rights(dir_fd, rights_base, rights_inheriting)
                .expect("droping fd_filestat_set_size inheriting right on a directory");
        }

        // Test that we can truncate the file without the
        // wasi_unstable::RIGHT_FD_FILESTAT_SET_SIZE right.
        file_fd = wasi::path_open(dir_fd, 0, "file", wasi::OFLAGS_TRUNC, 0, 0, 0)
            .expect("truncating a file without fd_filestat_set_size right");
        wasi::fd_close(file_fd).expect("closing a file");

        rights_base &= !wasi::RIGHTS_PATH_FILESTAT_SET_SIZE;
        wasi::fd_fdstat_set_rights(dir_fd, rights_base, rights_inheriting)
            .expect("droping path_filestat_set_size base right on a directory");

        // Test that clearing wasi_unstable::RIGHT_PATH_FILESTAT_SET_SIZE actually
        // took effect.
        dir_fdstat = wasi::fd_fdstat_get(dir_fd).expect("reading the fdstat from a directory");
        assert_eq!(
            (dir_fdstat.fs_rights_base & wasi::RIGHTS_PATH_FILESTAT_SET_SIZE),
            0,
            "reading the fdstat from a directory",
        );

        // Test that we can't truncate the file without the
        // wasi_unstable::RIGHT_PATH_FILESTAT_SET_SIZE right.
        assert_errno!(
            wasi::path_open(dir_fd, 0, "file", wasi::OFLAGS_TRUNC, 0, 0, 0)
                .expect_err("truncating a file without path_filestat_set_size right")
                .raw_error(),
            wasi::ERRNO_NOTCAPABLE
        );
    }

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
    unsafe { test_truncation_rights(dir_fd) }
}
