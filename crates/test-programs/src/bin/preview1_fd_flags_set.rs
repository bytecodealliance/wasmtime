use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_fd_fdstat_set_flags(dir_fd: wasip1::Fd) {
    const FILE_NAME: &str = "file";
    let data = &[0u8; 100];

    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        FILE_NAME,
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_READ | wasip1::RIGHTS_FD_WRITE,
        0,
        wasip1::FDFLAGS_APPEND,
    )
    .expect("opening a file");

    // Write some data and then verify the written data
    assert_eq!(
        wasip1::fd_write(
            file_fd,
            &[wasip1::Ciovec {
                buf: data.as_ptr(),
                buf_len: data.len(),
            }],
        )
        .expect("writing to a file"),
        data.len(),
        "should write {} bytes",
        data.len(),
    );

    wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_SET).expect("seeking file");

    let buffer = &mut [0u8; 100];

    assert_eq!(
        wasip1::fd_read(
            file_fd,
            &[wasip1::Iovec {
                buf: buffer.as_mut_ptr(),
                buf_len: buffer.len(),
            }]
        )
        .expect("reading file"),
        buffer.len(),
        "should read {} bytes",
        buffer.len()
    );

    assert_eq!(&data[..], &buffer[..]);

    let data = &[1u8; 100];

    // Seek back to the start to ensure we're in append-only mode
    wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_SET).expect("seeking file");

    assert_eq!(
        wasip1::fd_write(
            file_fd,
            &[wasip1::Ciovec {
                buf: data.as_ptr(),
                buf_len: data.len(),
            }],
        )
        .expect("writing to a file"),
        data.len(),
        "should write {} bytes",
        data.len(),
    );

    wasip1::fd_seek(file_fd, 100, wasip1::WHENCE_SET).expect("seeking file");

    assert_eq!(
        wasip1::fd_read(
            file_fd,
            &[wasip1::Iovec {
                buf: buffer.as_mut_ptr(),
                buf_len: buffer.len(),
            }]
        )
        .expect("reading file"),
        buffer.len(),
        "should read {} bytes",
        buffer.len()
    );

    assert_eq!(&data[..], &buffer[..]);

    wasip1::fd_fdstat_set_flags(file_fd, 0).expect("disabling flags");

    // Overwrite some existing data to ensure the append mode is now off
    wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_SET).expect("seeking file");

    let data = &[2u8; 100];

    assert_eq!(
        wasip1::fd_write(
            file_fd,
            &[wasip1::Ciovec {
                buf: data.as_ptr(),
                buf_len: data.len(),
            }],
        )
        .expect("writing to a file"),
        data.len(),
        "should write {} bytes",
        data.len(),
    );

    wasip1::fd_seek(file_fd, 0, wasip1::WHENCE_SET).expect("seeking file");

    assert_eq!(
        wasip1::fd_read(
            file_fd,
            &[wasip1::Iovec {
                buf: buffer.as_mut_ptr(),
                buf_len: buffer.len(),
            }]
        )
        .expect("reading file"),
        buffer.len(),
        "should read {} bytes",
        buffer.len()
    );

    assert_eq!(&data[..], &buffer[..]);

    wasip1::fd_close(file_fd).expect("close file");

    let stat = wasip1::path_filestat_get(dir_fd, 0, FILE_NAME).expect("stat path");

    assert_eq!(stat.size, 200, "expected a file size of 200");

    wasip1::path_unlink_file(dir_fd, FILE_NAME).expect("unlinking file");
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

    let dir_fd = match open_scratch_directory(&arg) {
        Ok(dir_fd) => dir_fd,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1)
        }
    };

    unsafe {
        test_fd_fdstat_set_flags(dir_fd);
    }
}
