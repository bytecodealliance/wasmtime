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

fn test_path_open_create_existing(dir: &Descriptor) {
    create_file(dir, "file");
    let err = dir
        .open_at(
            PathFlags::empty(),
            "file",
            OpenFlags::CREATE | OpenFlags::EXCLUSIVE,
            DescriptorFlags::empty(),
        )
        .expect_err("trying to create a file that already exists");
    assert!(matches!(err, ErrorCode::Exist), "unexpected error {err:?}");
    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_open_create_existing(dir)
}
