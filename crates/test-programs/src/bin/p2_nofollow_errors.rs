use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn test_nofollow_errors(dir: &Descriptor) {
    // Create a directory for the symlink to point to.
    dir.create_directory_at("target").expect("creating a dir");

    // Create a symlink.
    dir.symlink_at("target", "symlink")
        .expect("creating a symlink");

    // Try to open it as a directory with O_NOFOLLOW again.
    let err = dir
        .open_at(
            PathFlags::empty(),
            "symlink",
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .expect_err("opening a directory symlink as a directory should fail");
    assert!(
        matches!(err, ErrorCode::Loop | ErrorCode::NotDirectory),
        "unexpected error {err:?}"
    );

    // Try to open it with just O_NOFOLLOW.
    let err = dir
        .open_at(
            PathFlags::empty(),
            "symlink",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .expect_err("opening a symlink with O_NOFOLLOW should fail");
    assert!(
        matches!(err, ErrorCode::Loop | ErrorCode::Access),
        "unexpected error {err:?}"
    );

    // Try to open it as a directory without O_NOFOLLOW.
    let file = dir
        .open_at(
            PathFlags::SYMLINK_FOLLOW,
            "symlink",
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .expect("opening a symlink as a directory");
    drop(file);

    // Replace the target directory with a file.
    dir.unlink_file_at("symlink").expect("removing a file");
    dir.remove_directory_at("target")
        .expect("remove_directory on a directory should succeed");

    let file = dir
        .open_at(
            PathFlags::empty(),
            "target",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .expect("creating a file");
    drop(file);
    dir.symlink_at("target", "symlink")
        .expect("creating a symlink");

    // Try to open it as a directory with O_NOFOLLOW again.
    let err = dir
        .open_at(
            PathFlags::empty(),
            "symlink",
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .expect_err("opening a directory symlink as a directory should fail");
    assert!(
        matches!(err, ErrorCode::Loop | ErrorCode::NotDirectory),
        "unexpected error {err:?}"
    );

    // Try to open it with just O_NOFOLLOW.
    let err = dir
        .open_at(
            PathFlags::empty(),
            "symlink",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .expect_err("opening a symlink with NOFOLLOW should fail");
    assert!(matches!(err, ErrorCode::Loop), "unexpected error {err:?}");

    // Try to open it as a directory without O_NOFOLLOW.
    let err = dir
        .open_at(
            PathFlags::SYMLINK_FOLLOW,
            "symlink",
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .expect_err("opening a symlink to a file as a directory");
    assert!(
        matches!(err, ErrorCode::NotDirectory),
        "unexpected error {err:?}"
    );

    // Clean up.
    dir.unlink_file_at("target").expect("removing a file");
    dir.unlink_file_at("symlink").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_nofollow_errors(dir)
}
