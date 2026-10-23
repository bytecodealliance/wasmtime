use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn test_dirfd_not_dir(dir: &Descriptor) {
    // Open a file.
    let file = dir
        .open_at(
            PathFlags::empty(),
            "file",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .expect("opening a file");
    // Now try to open a file underneath it as if it were a directory.
    let err = file
        .open_at(
            PathFlags::empty(),
            "foo",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .expect_err("non-directory base descriptor should get not-directory");
    assert!(
        matches!(err, ErrorCode::NotDirectory),
        "unexpected error {err:?}"
    );
    drop(file);
    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_dirfd_not_dir(dir)
}
