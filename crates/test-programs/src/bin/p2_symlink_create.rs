use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, OpenFlags, PathFlags};

fn create_symlink_to_file(dir: &Descriptor) {
    // Create a file for the symlink to point to.
    let target = dir
        .open_at(
            PathFlags::empty(),
            "target",
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .expect("creating a file");
    drop(target);

    // Create a symlink.
    dir.symlink_at("target", "symlink")
        .expect("creating a symlink");

    // Try to open it as a file following symlinks.
    let target_file_via_symlink = dir
        .open_at(
            PathFlags::SYMLINK_FOLLOW,
            "symlink",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .expect("opening a symlink as a file");
    drop(target_file_via_symlink);

    // Clean up.
    dir.unlink_file_at("symlink").expect("removing the symlink");
    dir.unlink_file_at("target")
        .expect("removing the target file");
}

fn create_symlink_to_directory(dir: &Descriptor) {
    // Create a directory for the symlink to point to.
    dir.create_directory_at("target").expect("creating a dir");

    // Create a symlink.
    dir.symlink_at("target", "symlink")
        .expect("creating a symlink");

    // Try to open it as a directory following symlinks.
    let target_dir_via_symlink = dir
        .open_at(
            PathFlags::SYMLINK_FOLLOW,
            "symlink",
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .expect("opening a symlink as a directory");
    drop(target_dir_via_symlink);

    // Clean up.
    dir.unlink_file_at("symlink")
        .expect("remove symlink to directory");
    dir.remove_directory_at("target")
        .expect("remove_directory on a directory should succeed");
}

fn create_symlink_to_root(dir: &Descriptor) {
    // Create a symlink.
    dir.symlink_at("/", "symlink")
        .expect_err("creating a symlink to an absolute path");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    create_symlink_to_file(dir);
    create_symlink_to_directory(dir);
    create_symlink_to_root(dir);
}
