/// An output stream should trap when it is asked to write more bytes than
/// `check-write` permitted.
fn main() {
    let stdout = test_programs::wasi::cli::stdout::get_stdout();

    let permit = stdout.check_write().unwrap();
    assert!(permit > 0);

    // This should trap according to the `wasi:io/streams` spec since we're
    // trying to write more bytes than `check-write` gave permission for:
    let contents = vec![0; usize::try_from(permit + 1).unwrap()];
    stdout.write(&contents).unwrap();
    unreachable!("attempt to write more bytes than permitted should have trapped");
}
