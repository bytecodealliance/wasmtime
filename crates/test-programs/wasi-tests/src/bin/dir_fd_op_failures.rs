use std::{env, process};
use wasi_tests::open_scratch_directory;

unsafe fn test_fd_dir_ops(dir_fd: wasi::Fd) {
    let stat = wasi::fd_filestat_get(dir_fd).expect("failed to fdstat");
    assert_eq!(stat.filetype, wasi::FILETYPE_DIRECTORY);

    let mut read_buf = vec![0; 128].into_boxed_slice();
    let iovec = wasi::Iovec {
        buf: read_buf.as_mut_ptr(),
        buf_len: read_buf.len(),
    };
    let r = wasi::fd_read(dir_fd, &[iovec]);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_read error");

    let r = wasi::fd_pread(dir_fd, &[iovec], 0);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_pread error");

    let write_buf = vec![0; 128].into_boxed_slice();
    let ciovec = wasi::Ciovec {
        buf: write_buf.as_ptr(),
        buf_len: write_buf.len(),
    };
    let r = wasi::fd_write(dir_fd, &[ciovec]);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_write error");

    let r = wasi::fd_pwrite(dir_fd, &[ciovec], 0);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_pwrite error");

    let r = wasi::fd_seek(dir_fd, 0, wasi::WHENCE_CUR);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_seek WHENCE_CUR error");
    let r = wasi::fd_seek(dir_fd, 0, wasi::WHENCE_SET);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_seek WHENCE_SET error");
    let r = wasi::fd_seek(dir_fd, 0, wasi::WHENCE_END);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_seek WHENCE_END error");

    let r = wasi::fd_tell(dir_fd);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_tell error");

    let r = wasi::fd_advise(dir_fd, 0, 0, wasi::ADVICE_DONTNEED);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_advise error");

    let r = wasi::fd_allocate(dir_fd, 0, 0);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_allocate error");

    let r = wasi::fd_datasync(dir_fd);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_datasync error");

    let r = wasi::fd_sync(dir_fd);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_sync error");

    let r = wasi::fd_fdstat_set_flags(dir_fd, wasi::FDFLAGS_NONBLOCK);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_fdstat_set_flags error");

    let r = wasi::fd_filestat_set_size(dir_fd, 0);
    assert_eq!(r, Err(wasi::ERRNO_BADF), "fd_filestat_set_size error");
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

    unsafe {
        test_fd_dir_ops(dir_fd);
    }
}
