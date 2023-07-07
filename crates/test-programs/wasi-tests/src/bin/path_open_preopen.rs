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
