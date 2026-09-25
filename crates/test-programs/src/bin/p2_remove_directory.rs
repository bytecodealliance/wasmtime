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

fn test_remove_directory(dir: &Descriptor) {
    // Create a directory in the scratch directory.
    dir.create_directory_at("dir")
        .expect("creating a directory");

    // Test that removing it succeeds.
    dir.remove_directory_at("dir")
        .expect("remove_directory on a directory should succeed");

    // There isn't consistient behavior across operating systems of whether removing with a
    // directory where the path has a trailing slash succeeds or fails, so we won't test
    // that behavior.

    // Create a temporary file.
    create_file(dir, "file");

    // Test that removing it with no trailing slash fails.
    let err = dir
        .remove_directory_at("file")
        .expect_err("remove_directory without a trailing slash on a file should fail");
    assert!(
        matches!(err, ErrorCode::NotDirectory),
        "unexpected error {err:?}"
    );

    // Test that removing it with a trailing slash fails.
    let err = dir
        .remove_directory_at("file/")
        .expect_err("remove_directory with a trailing slash on a file should fail");
    assert!(
        matches!(err, ErrorCode::NotDirectory),
        "unexpected error {err:?}"
    );

    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_remove_directory(dir)
}
