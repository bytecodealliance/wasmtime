use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn test_dangling_symlink(dir: &Descriptor) {
    // First create a dangling symlink. Not all platforms (e.g. Windows) support
    // dangling symlinks, so skip the test if creation fails.
    if dir.symlink_at("target", "symlink").is_err() {
        return;
    }

    // Try to open it as a directory with O_NOFOLLOW.
    let err = dir
        .open_at(
            PathFlags::empty(),
            "symlink",
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .expect_err("opening a dangling symlink as a directory");
    assert!(
        matches!(
            err,
            ErrorCode::NotDirectory | ErrorCode::Loop | ErrorCode::NoEntry
        ),
        "unexpected error {err:?}"
    );

    // Try to open it as a file with O_NOFOLLOW.
    let err = dir
        .open_at(
            PathFlags::empty(),
            "symlink",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .expect_err("opening a dangling symlink as a file");
    assert!(
        matches!(err, ErrorCode::Loop | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Clean up.
    dir.unlink_file_at("symlink")
        .expect("failed to remove file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_dangling_symlink(dir)
}
