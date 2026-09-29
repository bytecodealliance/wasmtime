//! Stream open paths must return `not-permitted` when the preopen denies the
//! access mode, matching non-stream `read`/`write` (not `bad-descriptor`).

use test_programs::wasi::filesystem::preopens;
use test_programs::wasi::filesystem::types::{DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn main() {
    let preopens = preopens::get_directories();

    // --- readonly preopen: write/append streams denied, read stream allowed ---
    let (readonly_dir, _) = preopens
        .iter()
        .find(|(_, path)| path == "readonly")
        .expect("find preopen named readonly");

    let file = readonly_dir
        .open_at(
            PathFlags::empty(),
            "stream-perms.txt",
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .expect("open for reading");

    let err = file
        .write_via_stream(0)
        .expect_err("write_via_stream without write permission");
    assert_eq!(
        err,
        ErrorCode::NotPermitted,
        "write_via_stream should return not-permitted, got {err:?}"
    );

    let err = file
        .append_via_stream()
        .expect_err("append_via_stream without write permission");
    assert_eq!(
        err,
        ErrorCode::NotPermitted,
        "append_via_stream should return not-permitted, got {err:?}"
    );

    let stream = file
        .read_via_stream(0)
        .expect("read_via_stream with read permission");
    let contents = stream.blocking_read(100).expect("read file contents");
    drop(stream);
    drop(file);
    assert_eq!(contents, b"stream permission test\n");
}
