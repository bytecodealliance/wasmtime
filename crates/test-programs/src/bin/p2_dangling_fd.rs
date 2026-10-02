use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, OpenFlags, PathFlags};

fn open(dir: &Descriptor, path: &str, oflags: OpenFlags) -> Descriptor {
    dir.open_at(PathFlags::empty(), path, oflags, DescriptorFlags::empty())
        .expect("failed to open")
}

fn test_dangling_fd(dir: &Descriptor) {
    // Create a file, open it, delete it without closing the handle,
    // and then try creating it again
    let fd = open(dir, "file", OpenFlags::CREATE);
    drop(fd);
    let file = open(dir, "file", OpenFlags::empty());
    // Not all platforms (e.g. Windows) support removing files while a handle
    // is still open, so skip the test if that fails.
    if dir.unlink_file_at("file").is_err() {
        drop(file);
        dir.unlink_file_at("file").expect("failed to unlink");
        return;
    }
    let fd = open(dir, "file", OpenFlags::CREATE);
    drop(fd);

    // Now, repeat the same process but for a directory
    dir.create_directory_at("subdir")
        .expect("failed to create dir");
    let subdir = open(dir, "subdir", OpenFlags::DIRECTORY);
    dir.remove_directory_at("subdir")
        .expect("failed to remove dir 2");
    dir.create_directory_at("subdir")
        .expect("failed to create dir 2");

    // Clean up.
    drop(file);
    drop(subdir);
    dir.unlink_file_at("file").expect("failed to unlink");
    dir.remove_directory_at("subdir")
        .expect("failed to remove dir");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_dangling_fd(dir)
}
