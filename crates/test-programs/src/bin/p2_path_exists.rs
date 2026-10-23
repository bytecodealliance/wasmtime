use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{
    Descriptor, DescriptorFlags, DescriptorType, OpenFlags, PathFlags,
};

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

fn test_path_exists(dir: &Descriptor) {
    // Create a temporary directory
    dir.create_directory_at("subdir").expect("create directory");

    // Check directory exists:
    let file_stat = dir
        .stat_at(PathFlags::empty(), "subdir")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::Directory);

    // Should still exist with symlink follow flag:
    let file_stat = dir
        .stat_at(PathFlags::SYMLINK_FOLLOW, "subdir")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::Directory);

    // Create a file:
    create_file(dir, "subdir/file");
    // Check directory exists:
    let file_stat = dir
        .stat_at(PathFlags::empty(), "subdir/file")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::RegularFile);

    // Should still exist with symlink follow flag:
    let file_stat = dir
        .stat_at(PathFlags::SYMLINK_FOLLOW, "subdir/file")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::RegularFile);

    // Create a symlink to a file:
    dir.symlink_at("subdir/file", "link1")
        .expect("create symlink");
    // Check symlink exists:
    let file_stat = dir
        .stat_at(PathFlags::empty(), "link1")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::SymbolicLink);

    // Should still exist with symlink follow flag, pointing to regular file
    let file_stat = dir
        .stat_at(PathFlags::SYMLINK_FOLLOW, "link1")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::RegularFile);

    // Create a symlink to a dir:
    dir.symlink_at("subdir", "link2").expect("create symlink");
    // Check symlink exists:
    let file_stat = dir
        .stat_at(PathFlags::empty(), "link2")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::SymbolicLink);

    // Should still exist with symlink follow flag, pointing to directory
    let file_stat = dir
        .stat_at(PathFlags::SYMLINK_FOLLOW, "link2")
        .expect("reading file stats");
    assert_eq!(file_stat.type_, DescriptorType::Directory);

    dir.unlink_file_at("link1").expect("clean up");
    dir.unlink_file_at("link2").expect("clean up");
    dir.unlink_file_at("subdir/file").expect("clean up");
    dir.remove_directory_at("subdir").expect("clean up");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_exists(dir)
}
