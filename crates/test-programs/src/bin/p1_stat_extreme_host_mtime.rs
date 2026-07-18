#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]

use std::{env, process};
use test_programs::preview1::open_scratch_directory;

const FILENAME: &str = "extreme.dat";
const EXPECTED: &[u8] = b"hello";

unsafe fn test_stat_extreme_host_mtime(dir_fd: wasip1::Fd) {
    let st = wasip1::path_filestat_get(dir_fd, 0, FILENAME).expect("path_filestat_get");
    assert_eq!(st.size, EXPECTED.len() as u64, "size");
    let _ = st.mtim;
    let _ = st.atim;

    let fd =
        wasip1::path_open(dir_fd, 0, FILENAME, 0, wasip1::RIGHTS_FD_READ, 0, 0).expect("path_open");
    let mut buf = [0u8; 16];
    let nread = wasip1::fd_read(
        fd,
        &[wasip1::Iovec {
            buf: buf.as_mut_ptr(),
            buf_len: buf.len(),
        }],
    )
    .expect("fd_read");
    assert_eq!(&buf[..nread], EXPECTED, "contents");
    wasip1::fd_close(fd).expect("fd_close");
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
            process::exit(1);
        }
    };

    unsafe { test_stat_extreme_host_mtime(dir_fd) }
}
