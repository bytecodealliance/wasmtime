use test_programs::wasi::filesystem::preopens;
use test_programs::wasi::filesystem::types::{DescriptorFlags, OpenFlags, PathFlags};

/// An output stream should trap when it is asked to write more bytes than
/// `check-write` permitted.
fn main() {
    let (dir, _) = preopens::get_directories().into_iter().next().unwrap();
    let file = dir
        .open_at(
            PathFlags::empty(),
            "write-too-much",
            OpenFlags::CREATE,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .unwrap();
    let out = file.write_via_stream(0).unwrap();

    let permit = out.check_write().unwrap();
    assert!(permit > 0);

    // This should trap according to the `wasi:io/streams` spec since we're
    // trying to write more bytes than `check-write` gave permission for:
    let contents = vec![0; usize::try_from(permit + 1).unwrap()];
    out.write(&contents).unwrap();
    unreachable!("attempt to write more bytes than permitted should have trapped");
}
