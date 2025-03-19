const FIRST_PREOPEN: u32 = 3;

unsafe fn path_open_preopen() {
    let prestat = wasip1::fd_prestat_get(FIRST_PREOPEN).expect("fd 3 is a preopen");
    assert_eq!(
        prestat.tag,
        wasip1::PREOPENTYPE_DIR.raw(),
        "prestat is a directory"
    );
    let mut dst = Vec::with_capacity(prestat.u.dir.pr_name_len);
    wasip1::fd_prestat_dir_name(FIRST_PREOPEN, dst.as_mut_ptr(), dst.capacity())
        .expect("get preopen dir name");
    dst.set_len(prestat.u.dir.pr_name_len);

    let fdstat = wasip1::fd_fdstat_get(FIRST_PREOPEN).expect("get fdstat");

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
    let _ = wasip1::path_open(
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
    let _ = wasip1::path_open(FIRST_PREOPEN, 0, ".", 0, 0, 0, 0).expect("open with empty rights");

    // Open OFLAGS_DIRECTORY with an empty set of rights:
    let _ = wasip1::path_open(FIRST_PREOPEN, 0, ".", wasip1::OFLAGS_DIRECTORY, 0, 0, 0)
        .expect("open with O_DIRECTORY empty rights");

    // Open OFLAGS_DIRECTORY with just the read right:
    let _ = wasip1::path_open(
        FIRST_PREOPEN,
        0,
        ".",
        wasip1::OFLAGS_DIRECTORY,
        wasip1::RIGHTS_FD_READ,
        0,
        0,
    )
    .expect("open with O_DIRECTORY and read right");

    if !test_programs::preview1::config().errno_expect_windows() {
        // Open OFLAGS_DIRECTORY and read/write rights should fail with isdir:
        let err = wasip1::path_open(
            FIRST_PREOPEN,
            0,
            ".",
            wasip1::OFLAGS_DIRECTORY,
            wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
            0,
            0,
        )
        .err()
        .expect("open with O_DIRECTORY and read/write should fail");
        assert_eq!(
            err,
            wasip1::ERRNO_ISDIR,
            "opening directory read/write should fail with ISDIR"
        );
    } else {
        // Open OFLAGS_DIRECTORY and read/write rights will succeed, only on windows:
        let _ = wasip1::path_open(
            FIRST_PREOPEN,
            0,
            ".",
            wasip1::OFLAGS_DIRECTORY,
            wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
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

fn directory_base_rights() -> Vec<(wasip1::Rights, &'static str)> {
    vec![
        (
            wasip1::RIGHTS_PATH_CREATE_DIRECTORY,
            "PATH_CREATE_DIRECTORY",
        ),
        (wasip1::RIGHTS_PATH_CREATE_FILE, "PATH_CREATE_FILE"),
        (wasip1::RIGHTS_PATH_LINK_SOURCE, "PATH_LINK_SOURCE"),
        (wasip1::RIGHTS_PATH_LINK_TARGET, "PATH_LINK_TARGET"),
        (wasip1::RIGHTS_PATH_OPEN, "PATH_OPEN"),
        (wasip1::RIGHTS_FD_READDIR, "FD_READDIR"),
        (wasip1::RIGHTS_PATH_READLINK, "PATH_READLINK"),
        (wasip1::RIGHTS_PATH_RENAME_SOURCE, "PATH_RENAME_SOURCE"),
        (wasip1::RIGHTS_PATH_RENAME_TARGET, "PATH_RENAME_TARGET"),
        (wasip1::RIGHTS_PATH_SYMLINK, "PATH_SYMLINK"),
        (
            wasip1::RIGHTS_PATH_REMOVE_DIRECTORY,
            "PATH_REMOVE_DIRECTORY",
        ),
        (wasip1::RIGHTS_PATH_UNLINK_FILE, "PATH_UNLINK_FILE"),
        (wasip1::RIGHTS_PATH_FILESTAT_GET, "PATH_FILESTAT_GET"),
        (
            wasip1::RIGHTS_PATH_FILESTAT_SET_TIMES,
            "PATH_FILESTAT_SET_TIMES",
        ),
        (wasip1::RIGHTS_FD_FILESTAT_GET, "FD_FILESTAT_GET"),
        (
            wasip1::RIGHTS_FD_FILESTAT_SET_TIMES,
            "FD_FILESTAT_SET_TIMES",
        ),
    ]
}

pub(crate) fn directory_inheriting_rights() -> Vec<(wasip1::Rights, &'static str)> {
    let mut rights = directory_base_rights();
    rights.extend_from_slice(&[
        (wasip1::RIGHTS_FD_DATASYNC, "FD_DATASYNC"),
        (wasip1::RIGHTS_FD_READ, "FD_READ"),
        (wasip1::RIGHTS_FD_SEEK, "FD_SEEK"),
        (wasip1::RIGHTS_FD_FDSTAT_SET_FLAGS, "FD_FDSTAT_SET_FLAGS"),
        (wasip1::RIGHTS_FD_SYNC, "FD_SYNC"),
        (wasip1::RIGHTS_FD_TELL, "FD_TELL"),
        (wasip1::RIGHTS_FD_WRITE, "FD_WRITE"),
        (wasip1::RIGHTS_FD_ADVISE, "FD_ADVISE"),
        (wasip1::RIGHTS_FD_ALLOCATE, "FD_ALLOCATE"),
        (wasip1::RIGHTS_FD_FILESTAT_GET, "FD_FILESTAT_GET"),
        (wasip1::RIGHTS_FD_FILESTAT_SET_SIZE, "FD_FILESTAT_SET_SIZE"),
        (
            wasip1::RIGHTS_FD_FILESTAT_SET_TIMES,
            "FD_FILESTAT_SET_TIMES",
        ),
        (wasip1::RIGHTS_POLL_FD_READWRITE, "POLL_FD_READWRITE"),
    ]);
    rights
}
