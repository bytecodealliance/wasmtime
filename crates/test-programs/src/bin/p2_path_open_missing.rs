use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn test_path_open_missing(dir: &Descriptor) {
    let err = dir
        .open_at(
            PathFlags::empty(),
            "file",
            OpenFlags::empty(), // not passing CREATE here
            DescriptorFlags::empty(),
        )
        .expect_err("trying to open a file that doesn't exist");
    assert!(
        matches!(err, ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_open_missing(dir)
}
