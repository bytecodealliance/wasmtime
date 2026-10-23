use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, ErrorCode};

fn test_remove_nonempty_directory(dir: &Descriptor) {
    // Create a directory in the scratch directory.
    dir.create_directory_at("dir")
        .expect("creating a directory");

    // Create a directory in the directory we just created.
    dir.create_directory_at("dir/nested")
        .expect("creating a subdirectory");

    // Test that attempting to unlink the first directory returns the expected error code.
    let err = dir
        .remove_directory_at("dir")
        .expect_err("remove_directory on a directory should return NotEmpty");
    assert!(
        matches!(err, ErrorCode::NotEmpty),
        "unexpected error {err:?}"
    );

    // Removing the directories.
    dir.remove_directory_at("dir/nested")
        .expect("remove_directory on a nested directory should succeed");
    dir.remove_directory_at("dir")
        .expect("removing a directory");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_remove_nonempty_directory(dir)
}
