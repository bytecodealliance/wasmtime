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

fn open(dir: &Descriptor, path: &str) -> Result<Descriptor, ErrorCode> {
    dir.open_at(
        PathFlags::empty(),
        path,
        OpenFlags::empty(),
        DescriptorFlags::empty(),
    )
}

fn test_interesting_paths(dir: &Descriptor, arg: &str) {
    // Create a directory in the scratch directory.
    dir.create_directory_at("dir").expect("creating dir");

    // Create a directory in the directory we just created.
    dir.create_directory_at("dir/nested")
        .expect("creating a nested dir");

    // Create a file in the nested directory.
    create_file(dir, "dir/nested/file");

    // Now open it with an absolute path.
    let err = open(dir, "/dir/nested/file").expect_err("opening a file with an absolute path");
    assert!(
        matches!(err, ErrorCode::NotPermitted),
        "unexpected error {err:?}"
    );

    // Now open it with a path containing "..".
    let file = open(dir, "dir/.//nested/../../dir/nested/../nested///./file")
        .expect("opening a file with \"..\" in the path");
    drop(file);

    // Now open it with a trailing NUL. Windows will allow this.
    let err = open(dir, "dir/nested/file\0").expect_err("opening a file with a trailing NUL");
    assert!(
        matches!(err, ErrorCode::Invalid | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Now open it with a trailing slash.
    let err = open(dir, "dir/nested/file/")
        .expect_err("opening a file with a trailing slash should fail");
    assert!(
        matches!(err, ErrorCode::NotDirectory | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Now open it with trailing slashes.
    let err = open(dir, "dir/nested/file///")
        .expect_err("opening a file with trailing slashes should fail");
    assert!(
        matches!(err, ErrorCode::NotDirectory | ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Now open the directory with a trailing slash.
    let file = open(dir, "dir/nested/").expect("opening a directory with a trailing slash");
    drop(file);

    // Now open the directory with trailing slashes.
    let file = open(dir, "dir/nested///").expect("opening a directory with trailing slashes");
    drop(file);

    // Now open it with a path containing too many ".."s.
    let bad_path = format!("dir/nested/../../../{arg}/dir/nested/file");
    let err = open(dir, &bad_path)
        .expect_err("opening a file with too many \"..\"s in the path should fail");
    assert!(
        matches!(err, ErrorCode::NotPermitted),
        "unexpected error {err:?}"
    );
    dir.unlink_file_at("dir/nested/file")
        .expect("unlink_file on a symlink should succeed");
    dir.remove_directory_at("dir/nested")
        .expect("remove_directory on a directory should succeed");
    dir.remove_directory_at("dir")
        .expect("remove_directory on a directory should succeed");
}

fn main() {
    let preopens = get_directories();
    let (dir, name) = &preopens[0];

    // Run the tests.
    test_interesting_paths(dir, name)
}
