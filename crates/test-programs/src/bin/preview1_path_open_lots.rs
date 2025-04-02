use std::{env, process};
use test_programs::preview1::{create_file, open_scratch_directory};

unsafe fn test_path_open_lots(dir_fd: wasip1::Fd) {
    create_file(dir_fd, "file");

    for _ in 0..2000 {
        let f_readonly = wasip1::path_open(dir_fd, 0, "file", 0, wasip1::RIGHTS_FD_READ, 0, 0)
            .expect("open file readonly");

        let buffer = &mut [0u8; 100];
        let iovec = wasip1::Iovec {
            buf: buffer.as_mut_ptr(),
            buf_len: buffer.len(),
        };
        let nread = wasip1::fd_read(f_readonly, &[iovec]).expect("reading readonly file");
        assert_eq!(nread, 0, "readonly file is empty");

        wasip1::fd_close(f_readonly).expect("close readonly");
    }

    for _ in 0..2000 {
        let f_readonly = wasip1::path_open(dir_fd, 0, "file", 0, wasip1::RIGHTS_FD_READ, 0, 0)
            .expect("open file readonly");

        let buffer = &mut [0u8; 100];
        let iovec = wasip1::Iovec {
            buf: buffer.as_mut_ptr(),
            buf_len: buffer.len(),
        };
        let nread = wasip1::fd_pread(f_readonly, &[iovec], 0).expect("reading readonly file");
        assert_eq!(nread, 0, "readonly file is empty");

        wasip1::fd_close(f_readonly).expect("close readonly");
    }

    for _ in 0..2000 {
        let f = wasip1::path_open(
            dir_fd,
            0,
            "file",
            0,
            wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
            0,
            0,
        )
        .unwrap();

        let buffer = &[0u8; 100];
        let ciovec = wasip1::Ciovec {
            buf: buffer.as_ptr(),
            buf_len: buffer.len(),
        };
        let nwritten = wasip1::fd_write(f, &[ciovec]).expect("write failed");
        assert_eq!(nwritten, 100);

        wasip1::fd_close(f).unwrap();
    }

    for _ in 0..2000 {
        let f = wasip1::path_open(
            dir_fd,
            0,
            "file",
            0,
            wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
            0,
            0,
        )
        .unwrap();

        let buffer = &[0u8; 100];
        let ciovec = wasip1::Ciovec {
            buf: buffer.as_ptr(),
            buf_len: buffer.len(),
        };
        let nwritten = wasip1::fd_pwrite(f, &[ciovec], 0).expect("write failed");
        assert_eq!(nwritten, 100);

        wasip1::fd_close(f).unwrap();
    }

    wasip1::path_unlink_file(dir_fd, "file").expect("removing a file");
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
    unsafe { test_path_open_lots(dir_fd) }
}
