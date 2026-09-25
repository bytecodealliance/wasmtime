use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn create_file(dir: &Descriptor, path: &str) {
    let file = dir
        .open_at(
            PathFlags::empty(),
            path,
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .expect("creating a file");
    drop(file);
}

fn test_unlink_file_trailing_slashes(dir: &Descriptor) {
    // Create a directory in the scratch directory.
    dir.create_directory_at("dir")
        .expect("creating a directory");

    // Test that unlinking it fails. macOS reports not-permitted, other unix
    // platforms report is-directory, and Windows reports access.
    let err = dir
        .unlink_file_at("dir")
        .expect_err("unlink_file on a directory should fail");
    assert!(
        matches!(
            err,
            ErrorCode::NotPermitted | ErrorCode::IsDirectory | ErrorCode::Access
        ),
        "unexpected error {err:?}"
    );

    // Test that unlinking it with a trailing flash fails.
    let err = dir
        .unlink_file_at("dir/")
        .expect_err("unlink_file on a directory should fail");
    assert!(
        matches!(
            err,
            ErrorCode::NotPermitted | ErrorCode::IsDirectory | ErrorCode::Access
        ),
        "unexpected error {err:?}"
    );

    // Clean up.
    dir.remove_directory_at("dir")
        .expect("removing a directory");

    // Create a temporary file.
    create_file(dir, "file");

    // Test that unlinking it with a trailing flash fails.
    let err = dir
        .unlink_file_at("file/")
        .expect_err("unlink_file with a trailing slash should fail");
    assert!(
        matches!(err, ErrorCode::NotDirectory),
        "unexpected error {err:?}"
    );

    // Test that unlinking it with no trailing flash succeeds.
    dir.unlink_file_at("file")
        .expect("unlink_file with no trailing slash should succeed");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_unlink_file_trailing_slashes(dir)
}
