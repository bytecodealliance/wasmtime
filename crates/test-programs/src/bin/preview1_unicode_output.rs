use test_programs::preview1::STDOUT_FD;
fn main() {
    let text = "مرحبا بكم\n";

    let ciovecs = [wasi::Ciovec {
        buf: text.as_bytes().as_ptr(),
        buf_len: text.as_bytes().len(),
    }];
    let written = unsafe { wasi::fd_write(STDOUT_FD, &ciovecs) }.expect("write succeeds");
    assert_eq!(
        written,
        text.as_bytes().len(),
        "full contents should be written"
    );
}
