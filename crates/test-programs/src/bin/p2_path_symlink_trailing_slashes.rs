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

fn test_path_symlink_trailing_slashes(dir: &Descriptor) {
    // Dangling symlink: Without a trailing slash, this should succeed. Not
    // all platforms (e.g. Windows) support dangling symlinks though, so skip
    // the dangling tests if creation fails.
    if dir.symlink_at("source", "target").is_ok() {
        dir.unlink_file_at("target").expect("removing a file");

        // Dangling symlink: Link destination shouldn't end with a slash.
        let err = dir
            .symlink_at("source", "target/")
            .expect_err("link destination ending with a slash should fail");
        assert!(
            matches!(err, ErrorCode::NoEntry),
            "unexpected error {err:?}"
        );
    }

    // Link destination already exists, target has trailing slash.
    dir.create_directory_at("target")
        .expect("creating a directory");
    let err = dir
        .symlink_at("source", "target/")
        .expect_err("link destination already exists");
    assert!(
        matches!(err, ErrorCode::Exist | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );
    dir.remove_directory_at("target")
        .expect("removing a directory");

    // Link destination already exists, target has no trailing slash.
    dir.create_directory_at("target")
        .expect("creating a directory");
    let err = dir
        .symlink_at("source", "target")
        .expect_err("link destination already exists");
    assert!(
        matches!(err, ErrorCode::Exist | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );
    dir.remove_directory_at("target")
        .expect("removing a directory");

    // Link destination already exists, target has trailing slash.
    create_file(dir, "target");

    let err = dir
        .symlink_at("source", "target/")
        .expect_err("link destination already exists");
    assert!(
        matches!(err, ErrorCode::NotDirectory | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );
    dir.unlink_file_at("target").expect("removing a file");

    // Link destination already exists, target has no trailing slash.
    create_file(dir, "target");

    let err = dir
        .symlink_at("source", "target")
        .expect_err("link destination already exists");
    assert!(
        matches!(err, ErrorCode::Exist | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );
    dir.unlink_file_at("target").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_symlink_trailing_slashes(dir)
}
