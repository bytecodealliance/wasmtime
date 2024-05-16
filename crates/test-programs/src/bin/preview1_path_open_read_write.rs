use std::{env, process};
use test_programs::preview1::{assert_errno, create_file, open_scratch_directory};

unsafe fn test_path_open_read_write(dir_fd: wasi::Fd) {
    create_file(dir_fd, "file");

    let f_readonly = wasi::path_open(dir_fd, 0, "file", 0, wasi::RIGHTS_FD_READ, 0, 0)
        .expect("open file readonly");

    let stat = wasi::fd_fdstat_get(f_readonly).expect("get fdstat readonly");
    assert!(
        stat.fs_rights_base & wasi::RIGHTS_FD_READ == wasi::RIGHTS_FD_READ,
        "readonly has read right"
    );
    assert!(
        stat.fs_rights_base & wasi::RIGHTS_FD_WRITE == 0,
        "readonly does not have write right"
    );

    let buffer = &mut [0u8; 100];
    let iovec = wasi::Iovec {
        buf: buffer.as_mut_ptr(),
        buf_len: buffer.len(),
    };
    let nread = wasi::fd_read(f_readonly, &[iovec]).expect("reading readonly file");
    assert_eq!(nread, 0, "readonly file is empty");

    let write_buffer = &[1u8; 50];
    let ciovec = wasi::Ciovec {
        buf: write_buffer.as_ptr(),
        buf_len: write_buffer.len(),
    };
    // PERM is only the failure on windows under wasmtime-wasi. wasi-common
    // fails on windows with BADF, so we can't use the `windows =>` syntax
    // because that doesn't support alternatives like the agnostic syntax does.
    assert_errno!(
        wasi::fd_write(f_readonly, &[ciovec])
            .err()
            .expect("read of writeonly fails"),
        wasi::ERRNO_PERM,
        wasi::ERRNO_BADF
    );

    wasi::fd_close(f_readonly).expect("close readonly");

    // =============== WRITE ONLY ==================
    let f_writeonly = wasi::path_open(dir_fd, 0, "file", 0, wasi::RIGHTS_FD_WRITE, 0, 0)
        .expect("open file writeonly");

    let stat = wasi::fd_fdstat_get(f_writeonly).expect("get fdstat writeonly");
    assert!(
        stat.fs_rights_base & wasi::RIGHTS_FD_READ == 0,
        "writeonly does not have read right"
    );
    assert!(
        stat.fs_rights_base & wasi::RIGHTS_FD_WRITE == wasi::RIGHTS_FD_WRITE,
        "writeonly has write right"
    );

    // See above for description of PERM
    assert_errno!(
        wasi::fd_read(f_writeonly, &[iovec])
            .err()
            .expect("read of writeonly fails"),
        wasi::ERRNO_PERM,
        wasi::ERRNO_BADF
    );
    let bytes_written = wasi::fd_write(f_writeonly, &[ciovec]).expect("write to writeonly");
    assert_eq!(bytes_written, write_buffer.len());

    wasi::fd_close(f_writeonly).expect("close writeonly");

    // ============== READ WRITE =======================

    let f_readwrite = wasi::path_open(
        dir_fd,
        0,
        "file",
        0,
        wasi::RIGHTS_FD_READ | wasi::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("open file readwrite");
    let stat = wasi::fd_fdstat_get(f_readwrite).expect("get fdstat readwrite");
    assert!(
        stat.fs_rights_base & wasi::RIGHTS_FD_READ == wasi::RIGHTS_FD_READ,
        "readwrite has read right"
    );
    assert!(
        stat.fs_rights_base & wasi::RIGHTS_FD_WRITE == wasi::RIGHTS_FD_WRITE,
        "readwrite has write right"
    );

    let nread = wasi::fd_read(f_readwrite, &[iovec]).expect("reading readwrite file");
    assert_eq!(
        nread,
        write_buffer.len(),
        "readwrite file contains contents from writeonly open"
    );

    let write_buffer_2 = &[2u8; 25];
    let ciovec = wasi::Ciovec {
        buf: write_buffer_2.as_ptr(),
        buf_len: write_buffer_2.len(),
    };
    let bytes_written = wasi::fd_write(f_readwrite, &[ciovec]).expect("write to readwrite");
    assert_eq!(bytes_written, write_buffer_2.len());

    let filestat = wasi::fd_filestat_get(f_readwrite).expect("get filestat readwrite");
    assert_eq!(
        filestat.size as usize,
        write_buffer.len() + write_buffer_2.len(),
        "total written is both write buffers"
    );

    wasi::fd_close(f_readwrite).expect("close readwrite");

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
    unsafe { test_path_open_read_write(dir_fd) }
}
