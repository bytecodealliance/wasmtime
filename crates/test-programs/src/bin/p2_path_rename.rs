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

fn open(dir: &Descriptor, path: &str, oflags: OpenFlags) -> Result<Descriptor, ErrorCode> {
    dir.open_at(PathFlags::empty(), path, oflags, DescriptorFlags::empty())
}

fn rename(dir: &Descriptor, source: &str, target: &str) -> Result<(), ErrorCode> {
    dir.rename_at(source, dir, target)
}

fn test_path_rename(dir: &Descriptor) {
    // First, try renaming a dir to nonexistent path
    // Create source directory
    dir.create_directory_at("source")
        .expect("creating a directory");

    // Try renaming the directory
    rename(dir, "source", "target").expect("renaming a directory");

    // Check that source directory doesn't exist anymore
    let err = open(dir, "source", OpenFlags::DIRECTORY)
        .expect_err("opening a nonexistent path as a directory should fail");
    assert!(
        matches!(err, ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Check that target directory exists
    let fd =
        open(dir, "target", OpenFlags::DIRECTORY).expect("opening renamed path as a directory");

    drop(fd);
    dir.remove_directory_at("target")
        .expect("removing a directory");

    // Now, try renaming a dir to existing empty dir
    dir.create_directory_at("source")
        .expect("creating a directory");
    dir.create_directory_at("target")
        .expect("creating a directory");
    rename(dir, "source", "target").expect("renaming a directory");

    // Check that source directory doesn't exist anymore
    let err = open(dir, "source", OpenFlags::DIRECTORY)
        .expect_err("opening a nonexistent path as a directory");
    assert!(
        matches!(err, ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Check that target directory exists
    let fd =
        open(dir, "target", OpenFlags::DIRECTORY).expect("opening renamed path as a directory");

    drop(fd);
    dir.remove_directory_at("target")
        .expect("removing a directory");

    // Now, try renaming a dir to existing non-empty dir
    dir.create_directory_at("source")
        .expect("creating a directory");
    dir.create_directory_at("target")
        .expect("creating a directory");
    create_file(dir, "target/file");

    let err =
        rename(dir, "source", "target").expect_err("renaming directory to a nonempty directory");
    assert!(
        matches!(err, ErrorCode::NotEmpty),
        "unexpected error {err:?}"
    );

    // Try renaming dir to a file. Some platforms (e.g. Windows) allow this,
    // so accept either outcome.
    match rename(dir, "source", "target/file") {
        Ok(()) => {
            dir.remove_directory_at("target/file")
                .expect("removing a directory");
        }
        Err(err) => {
            assert!(
                matches!(err, ErrorCode::NotDirectory),
                "unexpected error {err:?}"
            );
            dir.unlink_file_at("target/file").expect("removing a file");
            dir.remove_directory_at("source")
                .expect("removing a directory");
        }
    }
    dir.remove_directory_at("target")
        .expect("removing a directory");

    // Now, try renaming a file to a nonexistent path
    create_file(dir, "source");
    rename(dir, "source", "target").expect("renaming a file");

    // Check that source file doesn't exist anymore
    let err = open(dir, "source", OpenFlags::empty())
        .expect_err("opening a nonexistent path should fail");
    assert!(
        matches!(err, ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Check that target file exists
    let fd = open(dir, "target", OpenFlags::empty()).expect("opening renamed path");

    drop(fd);
    dir.unlink_file_at("target").expect("removing a file");

    // Now, try renaming file to an existing file
    create_file(dir, "source");
    create_file(dir, "target");

    rename(dir, "source", "target").expect("renaming file to another existing file");

    // Check that source file doesn't exist anymore
    let err = open(dir, "source", OpenFlags::empty()).expect_err("opening a nonexistent path");
    assert!(
        matches!(err, ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Check that target file exists
    let fd = open(dir, "target", OpenFlags::empty()).expect("opening renamed path");

    drop(fd);
    dir.unlink_file_at("target").expect("removing a file");

    // Try renaming to an (empty) directory instead
    create_file(dir, "source");
    dir.create_directory_at("target")
        .expect("creating a directory");

    let err = rename(dir, "source", "target")
        .expect_err("renaming a file to existing directory should fail");
    assert!(
        matches!(err, ErrorCode::Access | ErrorCode::IsDirectory),
        "unexpected error {err:?}"
    );

    dir.remove_directory_at("target")
        .expect("removing a directory");
    dir.unlink_file_at("source").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_rename(dir)
}
