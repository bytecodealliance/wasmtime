#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, mem, process, slice, str};
use test_programs::preview1::{assert_errno, open_scratch_directory};

unsafe fn test_renumber(dir_fd: wasip1::Fd) {
    let pre_fd: wasip1::Fd = (libc::STDERR_FILENO + 1) as wasip1::Fd;

    assert!(dir_fd > pre_fd, "dir_fd number");

    // Create a file in the scratch directory.
    let fd_from = wasip1::path_open(
        dir_fd,
        0,
        "file1",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_from > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    // Get fd_from fdstat attributes
    let fdstat_from =
        wasip1::fd_fdstat_get(fd_from).expect("calling fd_fdstat on the open file descriptor");

    // Create another file in the scratch directory.
    let fd_to = wasip1::path_open(
        dir_fd,
        0,
        "file2",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_to > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    // Renumber fd of file1 into fd of file2
    wasip1::fd_renumber(fd_from, fd_to).expect("renumbering two descriptors");

    // Ensure that fd_from is closed
    assert_errno!(
        wasip1::fd_close(fd_from).expect_err("closing already closed file descriptor"),
        wasip1::ERRNO_BADF
    );

    // Ensure that fd_to is still open.
    let fdstat_to =
        wasip1::fd_fdstat_get(fd_to).expect("calling fd_fdstat on the open file descriptor");
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

    wasip1::fd_close(fd_to).expect("closing a file");

    wasip1::fd_renumber(0, 0).expect("renumbering 0 to 0");
    let fd_file3 = wasip1::path_open(
        dir_fd,
        0,
        "file3",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_file3 > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    assert_eq!(wasip1::fd_renumber(fd_file3, 127), Err(wasip1::ERRNO_BADF));
    assert_eq!(
        wasip1::fd_renumber(fd_file3, u32::MAX),
        Err(wasip1::ERRNO_BADF)
    );

    let fd_file4 = wasip1::path_open(
        dir_fd,
        0,
        "file4",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        fd_file4 > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );
}

unsafe fn test_renumber_loop(dir_fd: wasip1::Fd) {
    let mut cur = wasip1::path_open(
        dir_fd,
        0,
        "file1",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");

    // Historically `fd_renumber` didn't properly close the "to" file descriptor
    // which meant that the resources would be leaked. The test harness here
    // allows 1000 entries in the resource table, and then most unix systems
    // allow ~1000 fds open by default. Round that up and do this 2000 times to
    // try to exercise any bugs should they show up. Historically this would
    // exhaust host file descriptors before `fd_renumber` was fixed.
    for _ in 0..2000 {
        let next = wasip1::path_open(dir_fd, 0, "file1", 0, wasip1::RIGHTS_FD_READ, 0, 0)
            .expect("opening a file");
        wasip1::fd_renumber(cur, next).unwrap();
        cur = next;
    }

    wasip1::fd_close(cur).unwrap();
}

/// Renumbering a directory must not leave the `dirent_cache` attached to the
/// old descriptor. See #15327.
unsafe fn test_renumber_dirent_cache(dir_fd: wasip1::Fd) {
    for (dir, file) in [("cache-a", "from-a.txt"), ("cache-b", "from-b.txt")] {
        wasip1::path_create_directory(dir_fd, dir).expect("create a directory");
        let path = format!("{dir}/{file}");
        let fd = wasip1::path_open(
            dir_fd,
            0,
            &path,
            wasip1::OFLAGS_CREAT,
            wasip1::RIGHTS_FD_WRITE,
            0,
            0,
        )
        .expect("create a file");
        wasip1::fd_close(fd).expect("close a file");
    }

    let a = wasip1::path_open(dir_fd, 0, "cache-a", 0, 0, 0, 0).expect("open cache-a");
    let b = wasip1::path_open(dir_fd, 0, "cache-b", 0, 0, 0, 0).expect("open cache-b");

    // A small buffer leaves a partial entry in the adapter's global dirent
    // cache, associated with `a`.
    let mut small = [0u8; 23];
    wasip1::fd_readdir(a, small.as_mut_ptr(), small.len(), 0).expect("prime dirent cache");

    wasip1::fd_renumber(b, a).expect("renumber directories");

    let mut large = [0u8; 4096];
    let used =
        wasip1::fd_readdir(a, large.as_mut_ptr(), large.len(), 0).expect("readdir after renumber");

    let mut names = Vec::new();
    let mut off = 0;
    while off + mem::size_of::<wasip1::Dirent>() <= used {
        let dirent = (large.as_ptr().add(off) as *const wasip1::Dirent).read_unaligned();
        let name_off = off + mem::size_of::<wasip1::Dirent>();
        if name_off + dirent.d_namlen as usize > used {
            break;
        }
        let name = str::from_utf8(slice::from_raw_parts(
            large.as_ptr().add(name_off),
            dirent.d_namlen as usize,
        ))
        .expect("utf8 name");
        names.push(name.to_owned());
        off = name_off + dirent.d_namlen as usize;
    }

    assert!(
        names.iter().any(|name| name == "from-b.txt"),
        "expected entries from the renumbered directory, got {names:?}"
    );
    assert!(
        !names.iter().any(|name| name == "from-a.txt"),
        "stale directory stream from the old descriptor, got {names:?}"
    );

    wasip1::fd_close(a).expect("close a directory");
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
    unsafe { test_renumber(dir_fd) }
    unsafe { test_renumber_loop(dir_fd) }
    unsafe { test_renumber_dirent_cache(dir_fd) }
}
