use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_file_pread_pwrite(dir_fd: wasip1::Fd) {
    // Create a file in the scratch directory.
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        "file",
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    let contents = &[0u8, 1, 2, 3];
    let ciovec = wasip1::Ciovec {
        buf: contents.as_ptr() as *const _,
        buf_len: contents.len(),
    };
    let mut nwritten = wasip1::fd_pwrite(file_fd, &[ciovec], 0).expect("writing bytes at offset 0");
    assert_eq!(nwritten, 4, "nwritten bytes check");

    let contents = &mut [0u8; 4];
    let iovec = wasip1::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    let mut nread = wasip1::fd_pread(file_fd, &[iovec], 0).expect("reading bytes at offset 0");
    assert_eq!(nread, 4, "nread bytes check");
    assert_eq!(contents, &[0u8, 1, 2, 3], "written bytes equal read bytes");

    // Write all the data through multiple iovecs.
    //
    // Note that this needs to be done with a loop, because some
    // platforms do not support writing multiple iovecs at once.
    // See https://github.com/rust-lang/rust/issues/74825.
    let contents = &[0u8, 1, 2, 3];
    let mut offset = 0usize;
    loop {
        let mut ciovecs: Vec<wasip1::Ciovec> = Vec::new();
        let mut remaining = contents.len() - offset;
        if remaining > 2 {
            ciovecs.push(wasip1::Ciovec {
                buf: contents[offset..].as_ptr() as *const _,
                buf_len: 2,
            });
            remaining -= 2;
        }
        ciovecs.push(wasip1::Ciovec {
            buf: contents[contents.len() - remaining..].as_ptr() as *const _,
            buf_len: remaining,
        });

        nwritten = wasip1::fd_pwrite(file_fd, ciovecs.as_slice(), offset.try_into().unwrap())
            .expect("writing bytes at offset 0");

        offset += nwritten;
        if offset == contents.len() {
            break;
        }
    }
    assert_eq!(offset, 4, "nread bytes check");

    // Read all the data through multiple iovecs.
    //
    // Note that this needs to be done with a loop, because some
    // platforms do not support reading multiple iovecs at once.
    // See https://github.com/rust-lang/rust/issues/74825.
    let contents = &mut [0u8; 4];
    let mut offset = 0usize;
    loop {
        let buffer = &mut [0u8; 4];
        let iovecs = &[
            wasip1::Iovec {
                buf: buffer.as_mut_ptr() as *mut _,
                buf_len: 2,
            },
            wasip1::Iovec {
                buf: buffer[2..].as_mut_ptr() as *mut _,
                buf_len: 2,
            },
        ];
        nread = wasip1::fd_pread(file_fd, iovecs, offset as _).expect("reading bytes at offset 0");
        if nread == 0 {
            break;
        }
        contents[offset..offset + nread].copy_from_slice(&buffer[0..nread]);
        offset += nread;
    }
    assert_eq!(offset, 4, "nread bytes check");
    assert_eq!(contents, &[0u8, 1, 2, 3], "file cursor was overwritten");

    let contents = &mut [0u8; 4];
    let iovec = wasip1::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    nread = wasip1::fd_pread(file_fd, &[iovec], 2).expect("reading bytes at offset 2");
    assert_eq!(nread, 2, "nread bytes check");
    assert_eq!(contents, &[2u8, 3, 0, 0], "file cursor was overwritten");

    let contents = &[1u8, 0];
    let ciovec = wasip1::Ciovec {
        buf: contents.as_ptr() as *const _,
        buf_len: contents.len(),
    };
    nwritten = wasip1::fd_pwrite(file_fd, &[ciovec], 2).expect("writing bytes at offset 2");
    assert_eq!(nwritten, 2, "nwritten bytes check");

    let contents = &mut [0u8; 4];
    let iovec = wasip1::Iovec {
        buf: contents.as_mut_ptr() as *mut _,
        buf_len: contents.len(),
    };
    nread = wasip1::fd_pread(file_fd, &[iovec], 0).expect("reading bytes at offset 0");
    assert_eq!(nread, 4, "nread bytes check");
    assert_eq!(contents, &[0u8, 1, 1, 0], "file cursor was overwritten");

    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
}

unsafe fn test_file_pwrite_and_file_pos(dir_fd: wasip1::Fd) {
    let path = "file2";
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        path,
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("opening a file");
    assert!(
        file_fd > libc::STDERR_FILENO as wasip1::Fd,
        "file descriptor range check",
    );

    // Perform a 0-sized pwrite at an offset beyond the end of the file. Unix
    // semantics should pop out where nothing is actually written and the size
    // of the file isn't modified.
    assert_eq!(wasip1::fd_tell(file_fd).unwrap(), 0);
    let ciovec = wasip1::Ciovec {
        buf: [].as_ptr(),
        buf_len: 0,
    };
    let n = wasip1::fd_pwrite(file_fd, &[ciovec], 50).expect("writing bytes at offset 2");
    assert_eq!(n, 0);

    assert_eq!(wasip1::fd_tell(file_fd).unwrap(), 0);
    let stat = wasip1::fd_filestat_get(file_fd).unwrap();
    assert_eq!(stat.size, 0);

    // Now write a single byte and make sure it actually works
    let buf = [0];
    let ciovec = wasip1::Ciovec {
        buf: buf.as_ptr(),
        buf_len: buf.len(),
    };
    let n = wasip1::fd_pwrite(file_fd, &[ciovec], 50).expect("writing bytes at offset 50");
    assert_eq!(n, 1);

    assert_eq!(wasip1::fd_tell(file_fd).unwrap(), 0);
    let stat = wasip1::fd_filestat_get(file_fd).unwrap();
    assert_eq!(stat.size, 51);

    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, path).expect("removing a file");
}

unsafe fn test_file_pwrite_and_append(dir_fd: wasip1::Fd) {
    let path = "file3";
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        path,
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        wasip1::FDFLAGS_APPEND,
    )
    .expect("opening a file");

    // Inherit linux semantics for `pwrite` where if the file is opened in
    // append mode then the offset to `pwrite` is ignored.
    let buf = [0];
    let ciovec = wasip1::Ciovec {
        buf: buf.as_ptr(),
        buf_len: buf.len(),
    };
    let n = wasip1::fd_pwrite(file_fd, &[ciovec], 50).expect("writing bytes at offset 50");
    assert_eq!(n, 1);

    let stat = wasip1::fd_filestat_get(file_fd).unwrap();
    assert_eq!(stat.size, 1);

    let n = wasip1::fd_pwrite(file_fd, &[ciovec], 0).expect("writing bytes at offset 50");
    assert_eq!(n, 1);

    let stat = wasip1::fd_filestat_get(file_fd).unwrap();
    assert_eq!(stat.size, 2);

    wasip1::fd_close(file_fd).expect("closing a file");
    wasip1::path_unlink_file(dir_fd, path).expect("removing a file");
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
    unsafe {
        test_file_pread_pwrite(dir_fd);
        test_file_pwrite_and_file_pos(dir_fd);
        test_file_pwrite_and_append(dir_fd);
    }
}
