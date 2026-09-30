use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn test_symlink_loop(dir: &Descriptor) {
    // Create a self-referencing symlink. Not all platforms (e.g. Windows)
    // support dangling symlinks, so skip the test if creation fails.
    if dir.symlink_at("symlink", "symlink").is_err() {
        return;
    }

    // Try to open it.
    let err = dir
        .open_at(
            PathFlags::empty(),
            "symlink",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .expect_err("opening a self-referencing symlink");
    assert!(matches!(err, ErrorCode::Loop), "unexpected error {err:?}");

    // Clean up.
    dir.unlink_file_at("symlink").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_symlink_loop(dir)
}
