use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_fd_dir_ops(dir_fd: wasip1::Fd) {
    let stat = wasip1::fd_filestat_get(dir_fd).expect("failed to fdstat");
    assert_eq!(stat.filetype, wasip1::FILETYPE_DIRECTORY);

    let (pr_fd, pr_name_len) = (3..)
        .map_while(|fd| wasip1::fd_prestat_get(fd).ok().map(|stat| (fd, stat)))
        .find_map(|(fd, wasip1::Prestat { tag, u })| {
            (tag == wasip1::PREOPENTYPE_DIR.raw()).then_some((fd, u.dir.pr_name_len))
        })
        .expect("failed to find preopen directory");

    let mut pr_name = vec![];
    let r = wasip1::fd_prestat_dir_name(pr_fd, pr_name.as_mut_ptr(), 0);
    assert_eq!(r, Err(wasip1::ERRNO_NAMETOOLONG));

    // Test that passing a larger than necessary buffer works correctly
    let mut pr_name = vec![0; pr_name_len + 1];
    let r = wasip1::fd_prestat_dir_name(pr_fd, pr_name.as_mut_ptr(), pr_name_len + 1);
    assert_eq!(r, Ok(()));

    let mut read_buf = vec![0; 128].into_boxed_slice();
    let iovec = wasip1::Iovec {
        buf: read_buf.as_mut_ptr(),
        buf_len: read_buf.len(),
    };
    let r = wasip1::fd_read(dir_fd, &[iovec]);
    // On posix, this fails with ERRNO_ISDIR:
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_read error");

    let r = wasip1::fd_pread(dir_fd, &[iovec], 0);
    // On posix, this fails with ERRNO_ISDIR
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_pread error");

    let write_buf = vec![0; 128].into_boxed_slice();
    let ciovec = wasip1::Ciovec {
        buf: write_buf.as_ptr(),
        buf_len: write_buf.len(),
    };
    let r = wasip1::fd_write(dir_fd, &[ciovec]);
    // Same behavior as specified by POSIX:
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_write error");

    let r = wasip1::fd_pwrite(dir_fd, &[ciovec], 0);
    // Same behavior as specified by POSIX:
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_pwrite error");

    // Divergence from posix: lseek(dirfd) will return 0
    let r = wasip1::fd_seek(dir_fd, 0, wasip1::WHENCE_CUR);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_seek WHENCE_CUR error");
    let r = wasip1::fd_seek(dir_fd, 0, wasip1::WHENCE_SET);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_seek WHENCE_SET error");
    let r = wasip1::fd_seek(dir_fd, 0, wasip1::WHENCE_END);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_seek WHENCE_END error");

    // Tell isn't in posix, its basically lseek with WHENCE_CUR above
    let r = wasip1::fd_tell(dir_fd);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_tell error");

    // posix_fadvise(dirfd, 0, 0, POSIX_FADV_DONTNEED) will return 0 on linux.
    // not available on mac os.
    let r = wasip1::fd_advise(dir_fd, 0, 0, wasip1::ADVICE_DONTNEED);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_advise error");

    // fallocate(dirfd, FALLOC_FL_ZERO_RANGE, 0, 1) will fail with errno EBADF on linux.
    // not available on mac os.
    let r = wasip1::fd_allocate(dir_fd, 0, 0);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_allocate error");

    // fdatasync(dirfd) will return 0 on linux.
    // not available on mac os.
    let r = wasip1::fd_datasync(dir_fd);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_datasync error");

    // fsync(dirfd) will return 0 on linux.
    // not available on mac os.
    let r = wasip1::fd_sync(dir_fd);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_sync error");

    // fcntl(dirfd,  F_SETFL, O_NONBLOCK) will return 0 on linux.
    // not available on mac os.
    let r = wasip1::fd_fdstat_set_flags(dir_fd, wasip1::FDFLAGS_NONBLOCK);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_fdstat_set_flags error");

    // ftruncate(dirfd, 1) will fail with errno EINVAL on posix.
    // here, we fail with EBADF instead:
    let r = wasip1::fd_filestat_set_size(dir_fd, 0);
    assert_eq!(r, Err(wasip1::ERRNO_BADF), "fd_filestat_set_size error");
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

    unsafe {
        test_fd_dir_ops(dir_fd);
    }
}
