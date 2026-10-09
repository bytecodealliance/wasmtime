use test_programs::wasi::filesystem::preopens;
use test_programs::wasi::filesystem::types::{DescriptorFlags, OpenFlags, PathFlags};

/// A zero-length `write` is within any `check-write` grant (`0 <= n`), so it is
/// a no-op and must not trap, even while a previous write is still in flight.
fn main() {
    let (dir, _) = preopens::get_directories().into_iter().next().unwrap();
    let file = dir
        .open_at(
            PathFlags::empty(),
            "empty-write",
            OpenFlags::CREATE,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .unwrap();
    let out = file.write_via_stream(0).unwrap();

    let permit = out.check_write().unwrap();
    assert!(permit > 0);

    // Occupy the permit so `check-write` reports 0 and a write is in flight.
    out.write(&[0]).unwrap();
    assert_eq!(out.check_write().unwrap(), 0);

    out.write(&[]).unwrap();
    out.write_zeroes(0).unwrap();
}
