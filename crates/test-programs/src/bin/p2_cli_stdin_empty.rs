use test_programs::preview1::STDIN_FD;

fn main() {
    let mut buffer = [0_u8; 0];

    unsafe {
        wasip1::fd_read(
            STDIN_FD,
            &[wasip1::Iovec {
                buf: buffer.as_mut_ptr(),
                buf_len: 0,
            }],
        )
        .expect("empty read should succeed");
    }
}
