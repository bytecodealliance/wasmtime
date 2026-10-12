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

    // Test that removing it with trailing slashes also succeeds.
    dir.create_directory_at("dir")
        .expect("creating a directory");
    dir.remove_directory_at("dir/")
        .expect("remove_directory with a trailing slash on a directory should succeed");
    dir.create_directory_at("dir")
        .expect("creating a directory");
    dir.remove_directory_at("dir//")
        .expect("remove_directory with trailing slashes on a directory should succeed");

    // Test that removing a directory through a symlink with a trailing slash
    // fails, like on Linux, and leaves both in place.
    dir.create_directory_at("dir")
        .expect("creating a directory");
    if dir.symlink_at("dir", "dir_link").is_ok() {
        let err = dir
            .remove_directory_at("dir_link/")
            .expect_err("remove_directory through a symlink should fail");
        assert!(
            matches!(err, ErrorCode::NotDirectory),
            "unexpected error {err:?}"
        );
        dir.unlink_file_at("dir_link").expect("removing a symlink");
    }
    dir.remove_directory_at("dir")
        .expect("remove_directory on a directory should succeed");

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
