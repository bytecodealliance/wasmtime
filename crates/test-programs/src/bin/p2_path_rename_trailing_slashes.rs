use test_programs::preview1::config;
use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn assert_not_directory(dir: &Descriptor, old_path: &str, new_path: &str) {
    let err = match dir.rename_at(old_path, dir, new_path) {
        Ok(()) => panic!("renaming {old_path:?} to {new_path:?} should fail"),
        Err(err) => err,
    };
    assert!(
        matches!(err, ErrorCode::NotDirectory),
        "unexpected error {err:?} renaming {old_path:?} to {new_path:?}"
    );
}

fn test_path_rename_trailing_slashes(dir: &Descriptor) {
    // A directory with a trailing slash on either path is renamed.
    dir.create_directory_at("source")
        .expect("creating a directory");
    dir.rename_at("source/", dir, "target")
        .expect("renaming a directory with a trailing slash in the source name");
    dir.rename_at("target", dir, "source/")
        .expect("renaming a directory with a trailing slash in the destination name");
    dir.rename_at("source/", dir, "target/")
        .expect("renaming a directory with a trailing slash in the source and destination names");
    dir.rename_at("target", dir, "source")
        .expect("renaming a directory with no trailing slashes at all should work");
    dir.remove_directory_at("source")
        .expect("removing the directory");

    // A symlink to a directory outside of the preopen with a trailing slash on
    // either path isn't followed, so the directory isn't moved into the
    // preopen. The host checks that it's still in place. Windows can't create
    // such a symlink in the first place.
    match dir.symlink_at("../outside", "up") {
        Ok(()) => {
            assert_not_directory(dir, "up/", "stolen");
            assert_not_directory(dir, "up", "stolen/");
            dir.unlink_file_at("up").expect("removing a symlink");
        }
        Err(ErrorCode::NotPermitted) if config().errno_expect_windows() => {}
        Err(err) => panic!("creating a symlink: {err:?}"),
    }

    // A symlink to a directory inside of the preopen with a trailing slash on
    // either path isn't followed either.
    dir.create_directory_at("dir")
        .expect("creating a directory");
    dir.symlink_at("dir", "link").expect("creating a symlink");
    assert_not_directory(dir, "link/", "target");
    assert_not_directory(dir, "link", "target/");
    dir.stat_at(PathFlags::empty(), "dir")
        .expect("the directory wasn't renamed");
    dir.stat_at(PathFlags::empty(), "link")
        .expect("the symlink wasn't renamed");
    dir.unlink_file_at("link").expect("removing a symlink");
    dir.remove_directory_at("dir")
        .expect("removing a directory");

    // A file with a trailing slash on either path isn't a directory.
    dir.open_at(
        PathFlags::empty(),
        "file",
        OpenFlags::CREATE,
        DescriptorFlags::empty(),
    )
    .expect("creating a file");
    assert_not_directory(dir, "file/", "target");
    assert_not_directory(dir, "file", "target/");
    dir.stat_at(PathFlags::empty(), "file")
        .expect("the file wasn't renamed");
    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    // This test program requires a preopen at the path "sandbox", next to an
    // "outside" directory which isn't preopened.
    let preopens = get_directories();
    let (dir, _) = preopens
        .iter()
        .find(|(_, path)| path == "sandbox")
        .expect("find preopen named sandbox");

    // Run the tests.
    test_path_rename_trailing_slashes(dir)
}
