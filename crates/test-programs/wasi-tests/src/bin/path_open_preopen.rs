const FIRST_PREOPEN: u32 = 3;

unsafe fn path_open_preopen() {
    let prestat = wasi::fd_prestat_get(FIRST_PREOPEN).expect("fd 3 is a preopen");
    assert_eq!(
        prestat.tag,
        wasi::PREOPENTYPE_DIR.raw(),
        "prestat is a directory"
    );
    let mut dst = Vec::with_capacity(prestat.u.dir.pr_name_len);
    wasi::fd_prestat_dir_name(FIRST_PREOPEN, dst.as_mut_ptr(), dst.capacity())
        .expect("get preopen dir name");
    dst.set_len(prestat.u.dir.pr_name_len);

    let fdstat = wasi::fd_fdstat_get(FIRST_PREOPEN).expect("get fdstat");

    println!(
        "preopen dir: {:?} base {:?} inheriting {:?}",
        String::from_utf8_lossy(&dst),
        fdstat.fs_rights_base,
        fdstat.fs_rights_inheriting
    );
    for (right, name) in directory_base_rights() {
        assert!(
            (fdstat.fs_rights_base & right) == right,
            "fs_rights_base does not have required right `{name}`"
        );
    }
    for (right, name) in directory_inheriting_rights() {
        assert!(
            (fdstat.fs_rights_inheriting & right) == right,
            "fs_rights_inheriting does not have required right `{name}`"
        );
    }

    // Open with same rights it has now:
    let _ = wasi::path_open(
        FIRST_PREOPEN,
        0,
        ".",
        0,
        fdstat.fs_rights_base,
        fdstat.fs_rights_inheriting,
        0,
    )
    .expect("open with same rights");

    // Open with an empty set of rights:
    let _ = wasi::path_open(FIRST_PREOPEN, 0, ".", 0, 0, 0, 0).expect("open with empty rights");

    // Open OFLAGS_DIRECTORY with an empty set of rights:
    let _ = wasi::path_open(FIRST_PREOPEN, 0, ".", wasi::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("open with O_DIRECTORY empty rights");

    // Open OFLAGS_DIRECTORY with just the read right:
    let _ = wasi::path_open(
        FIRST_PREOPEN,
        0,
        ".",
        wasi::OFLAGS_DIRECTORY,
        wasi::RIGHTS_FD_READ,
        0,
        0,
    )
    .expect("open with O_DIRECTORY and read right");

    if !wasi_tests::TESTCONFIG.errno_expect_windows() {
        // Open OFLAGS_DIRECTORY and read/write rights should fail with isdir:
        let err = wasi::path_open(
            FIRST_PREOPEN,
            0,
            ".",
            wasi::OFLAGS_DIRECTORY,
            wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
            0,
            0,
        )
        .err()
        .expect("open with O_DIRECTORY and read/write should fail");
        assert_eq!(
            err,
            wasi::ERRNO_ISDIR,
            "opening directory read/write should fail with ISDIR"
        );
    } else {
        // Open OFLAGS_DIRECTORY and read/write rights will succeed, only on windows:
        let _ = wasi::path_open(
            FIRST_PREOPEN,
            0,
            ".",
            wasi::OFLAGS_DIRECTORY,
            wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
            0,
            0,
        )
        .expect("open with O_DIRECTORY and read/write should succeed on windows");
    }
}

fn main() {
    unsafe {
        path_open_preopen();
    }
}

// Hard-code the set of rights expected for a preopened directory. This is
// more brittle than we wanted to test for, but various userland
// implementations expect (at least) this set of rights to be present on all
// directories:

fn directory_base_rights() -> Vec<(wasi::Rights, &'static str)> {
    vec![
        (wasi::RIGHTS_PATH_CREATE_DIRECTORY, "PATH_CREATE_DIRECTORY"),
        (wasi::RIGHTS_PATH_CREATE_FILE, "PATH_CREATE_FILE"),
        (wasi::RIGHTS_PATH_LINK_SOURCE, "PATH_LINK_SOURCE"),
        (wasi::RIGHTS_PATH_LINK_TARGET, "PATH_LINK_TARGET"),
        (wasi::RIGHTS_PATH_OPEN, "PATH_OPEN"),
        (wasi::RIGHTS_FD_READDIR, "FD_READDIR"),
        (wasi::RIGHTS_PATH_READLINK, "PATH_READLINK"),
        (wasi::RIGHTS_PATH_RENAME_SOURCE, "PATH_RENAME_SOURCE"),
        (wasi::RIGHTS_PATH_RENAME_TARGET, "PATH_RENAME_TARGET"),
        (wasi::RIGHTS_PATH_SYMLINK, "PATH_SYMLINK"),
        (wasi::RIGHTS_PATH_REMOVE_DIRECTORY, "PATH_REMOVE_DIRECTORY"),
        (wasi::RIGHTS_PATH_UNLINK_FILE, "PATH_UNLINK_FILE"),
        (wasi::RIGHTS_PATH_FILESTAT_GET, "PATH_FILESTAT_GET"),
        (
            wasi::RIGHTS_PATH_FILESTAT_SET_TIMES,
            "PATH_FILESTAT_SET_TIMES",
        ),
        (wasi::RIGHTS_FD_FILESTAT_GET, "FD_FILESTAT_GET"),
        (wasi::RIGHTS_FD_FILESTAT_SET_TIMES, "FD_FILESTAT_SET_TIMES"),
    ]
}

pub(crate) fn directory_inheriting_rights() -> Vec<(wasi::Rights, &'static str)> {
    let mut rights = directory_base_rights();
    rights.extend_from_slice(&[
        (wasi::RIGHTS_FD_DATASYNC, "FD_DATASYNC"),
        (wasi::RIGHTS_FD_READ, "FD_READ"),
        (wasi::RIGHTS_FD_SEEK, "FD_SEEK"),
        (wasi::RIGHTS_FD_FDSTAT_SET_FLAGS, "FD_FDSTAT_SET_FLAGS"),
        (wasi::RIGHTS_FD_SYNC, "FD_SYNC"),
        (wasi::RIGHTS_FD_TELL, "FD_TELL"),
        (wasi::RIGHTS_FD_WRITE, "FD_WRITE"),
        (wasi::RIGHTS_FD_ADVISE, "FD_ADVISE"),
        (wasi::RIGHTS_FD_ALLOCATE, "FD_ALLOCATE"),
        (wasi::RIGHTS_FD_FILESTAT_GET, "FD_FILESTAT_GET"),
        (wasi::RIGHTS_FD_FILESTAT_SET_SIZE, "FD_FILESTAT_SET_SIZE"),
        (wasi::RIGHTS_FD_FILESTAT_SET_TIMES, "FD_FILESTAT_SET_TIMES"),
        (wasi::RIGHTS_POLL_FD_READWRITE, "POLL_FD_READWRITE"),
    ]);
    rights
}
