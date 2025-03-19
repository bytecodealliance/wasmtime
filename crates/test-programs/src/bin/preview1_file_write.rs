use std::{env, process};
use test_programs::preview1::open_scratch_directory;

unsafe fn test_file_long_write(dir_fd: wasip1::Fd, filename: &str) {
    // Open a file for writing
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        filename,
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("creating a file for writing");

    let mut content = Vec::new();
    // 16 byte string, 4096 times, is 64k
    for n in 0..4096 {
        let chunk = format!("123456789 {n:05} ");
        assert_eq!(chunk.as_str().as_bytes().len(), 16);
        content.extend_from_slice(chunk.as_str().as_bytes());
    }

    // Write to the file
    let nwritten = wasip1::fd_write(
        file_fd,
        &[wasip1::Ciovec {
            buf: content.as_slice().as_ptr() as *const _,
            buf_len: content.len(),
        }],
    )
    .expect("writing file content");
    assert_eq!(nwritten, content.len(), "nwritten bytes check");

    let stat = wasip1::fd_filestat_get(file_fd).expect("reading file stats");
    assert_eq!(
        stat.size,
        content.len() as u64,
        "file should be size of content",
    );

    wasip1::fd_close(file_fd).expect("closing the file");
    // Open the file for reading
    let file_fd = wasip1::path_open(dir_fd, 0, filename, 0, wasip1::RIGHTS_FD_READ, 0, 0)
        .expect("open the file for reading");

    // Read the file's contents
    let buffer = &mut [0u8; 100];
    let nread = wasip1::fd_read(
        file_fd,
        &[wasip1::Iovec {
            buf: buffer.as_mut_ptr(),
            buf_len: buffer.len(),
        }],
    )
    .expect("reading first chunk file content");

    assert_eq!(nread, buffer.len(), "read first chunk");
    assert_eq!(
        buffer,
        &content[..buffer.len()],
        "contents of first read chunk"
    );

    let end_cursor = content.len() - buffer.len();
    wasip1::fd_seek(file_fd, end_cursor as i64, wasip1::WHENCE_SET)
        .expect("seeking to end of file minus buffer size");

    let nread = wasip1::fd_read(
        file_fd,
        &[wasip1::Iovec {
            buf: buffer.as_mut_ptr(),
            buf_len: buffer.len(),
        }],
    )
    .expect("reading end chunk of file content");

    assert_eq!(nread, buffer.len(), "read end chunk len");
    assert_eq!(buffer, &content[end_cursor..], "contents of end read chunk");

    wasip1::fd_close(file_fd).expect("closing the file");

    // Open a file for writing
    let filename = "test-zero-write-fails.txt";
    let file_fd = wasip1::path_open(
        dir_fd,
        0,
        filename,
        wasip1::OFLAGS_CREAT,
        wasip1::RIGHTS_FD_WRITE,
        0,
        0,
    )
    .expect("creating a file for writing");
    wasip1::fd_close(file_fd).expect("closing the file");
    let file_fd = wasip1::path_open(dir_fd, 0, filename, 0, wasip1::RIGHTS_FD_READ, 0, 0)
        .expect("opening a file for writing");
    let res = wasip1::fd_write(
        file_fd,
        &[wasip1::Ciovec {
            buf: 3 as *const u8,
            buf_len: 0,
        }],
    );
    assert!(
        res == Err(wasip1::ERRNO_BADF) || res == Err(wasip1::ERRNO_PERM),
        "bad result {res:?}"
    )
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
    unsafe { test_file_long_write(dir_fd, "long_write.txt") }
}
