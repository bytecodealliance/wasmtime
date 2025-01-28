use std::fs::OpenOptions;
use std::io::Read;

/// Assert that we can read from "miscellanous" devices such as /dev/zero on UNIX-alikes (assuming
/// /dev is passed as a preopen).
fn main() {
    let mut device = OpenOptions::new()
        .read(true)
        .open("zero")
        .expect("/dev/zero should be found and openable");
    let mut buffer = [1, 1];
    device
        .read_exact(&mut buffer)
        .expect("/dev/zero should be readable");
    assert_eq!(buffer, [0, 0]);
}
