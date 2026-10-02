use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{DescriptorFlags, DescriptorType, OpenFlags, PathFlags};

fn test_close_preopen() {
    let mut preopens = get_directories();
    let (preopen, _name) = preopens.remove(0);

    // Open a second handle to the preopened directory.
    let dir = preopen
        .open_at(
            PathFlags::empty(),
            ".",
            OpenFlags::DIRECTORY,
            DescriptorFlags::READ,
        )
        .expect("opening the scratch directory");

    // Try to close the preopened directory handle.
    drop(preopen);

    // Ensure that `dir` is still open.
    assert_eq!(
        dir.get_type().expect("failed get_type"),
        DescriptorType::Directory,
        "expected the scratch directory to be a directory",
    );

    // Ensure that the preopen itself is unaffected by closing a handle to it.
    let preopens = get_directories();
    assert_eq!(preopens.len(), 1);
    assert_eq!(
        preopens[0].0.get_type().expect("failed get_type"),
        DescriptorType::Directory,
    );
}

fn main() {
    // Run the tests.
    test_close_preopen()
}
