use wasip2::clocks::wall_clock::Datetime;
use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags};

fn time(t: Option<Datetime>) -> Option<(u64, u32)> {
    t.map(|t| (t.seconds, t.nanoseconds))
}

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

fn open(dir: &Descriptor, path: &str, oflags: OpenFlags) -> Descriptor {
    dir.open_at(PathFlags::empty(), path, oflags, DescriptorFlags::empty())
        .expect("opening a path")
}

fn link(dir: &Descriptor, old: &str, new_dir: &Descriptor, new: &str) -> Result<(), ErrorCode> {
    dir.link_at(PathFlags::empty(), old, new_dir, new)
}

// This is a macro so the panic line number shows us where things went wrong.
macro_rules! check_rights {
    ($orig:ident, $link:ident) => {
        let orig_stat = $orig.stat().expect("reading stat of the source");
        let link_stat = $link.stat().expect("reading stat of the link");
        assert_eq!(orig_stat.type_, link_stat.type_, "type should be equal");
        assert_eq!(
            orig_stat.link_count, link_stat.link_count,
            "link_count should be equal"
        );
        assert_eq!(orig_stat.size, link_stat.size, "size should be equal");
        assert_eq!(
            time(orig_stat.data_access_timestamp),
            time(link_stat.data_access_timestamp),
            "atim should be equal"
        );
        assert_eq!(
            time(orig_stat.data_modification_timestamp),
            time(link_stat.data_modification_timestamp),
            "mtim should be equal"
        );
        assert_eq!(
            time(orig_stat.status_change_timestamp),
            time(link_stat.status_change_timestamp),
            "ctim should be equal"
        );
        assert!($orig.is_same_object(&$link), "should be the same object");

        let orig_flags = $orig.get_flags().expect("reading flags of the source");
        let link_flags = $link.get_flags().expect("reading flags of the link");
        assert_eq!(orig_flags, link_flags, "flags should be equal");
        let orig_type = $orig.get_type().expect("reading type of the source");
        let link_type = $link.get_type().expect("reading type of the link");
        assert_eq!(orig_type, link_type, "type should be equal");
    };
}

// Extra drops are needed for Windows, which will not remove
// the directory until all handles are closed.
fn test_path_link(dir: &Descriptor) {
    // Create a file
    create_file(dir, "file");

    // Open a fresh descriptor to the file. We won't have write access that was implied by
    // CREATE above.
    let file = open(dir, "file", OpenFlags::empty());

    // Create a link in the same directory and compare rights
    link(dir, "file", dir, "link").expect("creating a link in the same directory");

    let link_fd = open(dir, "link", OpenFlags::empty());

    check_rights!(file, link_fd);
    drop(link_fd); // needed for Windows
    dir.unlink_file_at("link").expect("removing a link");

    // Create a link in a different directory and compare rights
    dir.create_directory_at("subdir")
        .expect("creating a subdirectory");
    let subdir = open(dir, "subdir", OpenFlags::DIRECTORY);
    link(dir, "file", &subdir, "link").expect("creating a link in subdirectory");
    let link_fd = open(&subdir, "link", OpenFlags::empty());
    check_rights!(file, link_fd);
    drop(link_fd); // needed for Windows
    subdir.unlink_file_at("link").expect("removing a link");
    drop(subdir); // needed for Windows
    dir.remove_directory_at("subdir")
        .expect("removing a subdirectory");

    // Create a link to a path that already exists
    create_file(dir, "link");

    let err =
        link(dir, "file", dir, "link").expect_err("creating a link to existing path should fail");
    assert!(matches!(err, ErrorCode::Exist), "unexpected error {err:?}");
    dir.unlink_file_at("link").expect("removing a file");

    // Create a link to itself
    let err = link(dir, "file", dir, "file").expect_err("creating a link to itself should fail");
    assert!(matches!(err, ErrorCode::Exist), "unexpected error {err:?}");

    // Create a link where target is a directory
    dir.create_directory_at("link").expect("creating a dir");

    let err = link(dir, "file", dir, "link")
        .expect_err("creating a link where target is a directory should fail");
    assert!(matches!(err, ErrorCode::Exist), "unexpected error {err:?}");
    dir.remove_directory_at("link").expect("removing a dir");

    // Create a link to a directory
    dir.create_directory_at("subdir")
        .expect("creating a subdirectory");
    let subdir = open(dir, "subdir", OpenFlags::DIRECTORY);

    let err =
        link(dir, "subdir", dir, "link").expect_err("creating a link to a directory should fail");
    assert!(
        matches!(err, ErrorCode::NotPermitted | ErrorCode::Access),
        "unexpected error {err:?}"
    );
    drop(subdir); // close subdir before deleting it
    dir.remove_directory_at("subdir")
        .expect("removing a subdirectory");

    // Create a link to a file with trailing slash
    let err = link(dir, "file", dir, "link/")
        .expect_err("creating a link to a file with trailing slash should fail");
    assert!(
        matches!(err, ErrorCode::NoEntry),
        "unexpected error {err:?}"
    );

    // Not all platforms (e.g. Windows) support dangling symlinks, so skip the
    // rest of the test if creation fails.
    if dir.symlink_at("target", "symlink").is_ok() {
        // Create a link to a dangling symlink. This should succeed, because
        // we're not following symlinks
        link(dir, "symlink", dir, "link")
            .expect("creating a link to a dangling symlink should succeed");
        dir.unlink_file_at("symlink").expect("removing a symlink");
        dir.unlink_file_at("link").expect("removing a hardlink");

        // Create a link to a symlink loop
        dir.symlink_at("symlink", "symlink")
            .expect("creating a symlink loop");

        link(dir, "symlink", dir, "link")
            .expect("creating a link to a symlink loop should succeed");
        dir.unlink_file_at("symlink").expect("removing a symlink");
        dir.unlink_file_at("link").expect("removing a hardlink");

        // Create a link where target is a dangling symlink
        dir.symlink_at("target", "symlink")
            .expect("creating a dangling symlink");

        let err = link(dir, "file", dir, "symlink")
            .expect_err("creating a link where target is a dangling symlink");
        assert!(matches!(err, ErrorCode::Exist), "unexpected error {err:?}");
        dir.unlink_file_at("symlink").expect("removing a symlink");

        // Create a link where target is a dangling symlink following symlinks
        dir.symlink_at("target", "symlink")
            .expect("creating a dangling symlink");

        // Symlink following with link_at is rejected
        let err = dir
            .link_at(PathFlags::SYMLINK_FOLLOW, "symlink", dir, "link")
            .expect_err("calling link_at with SYMLINK_FOLLOW should fail");
        assert!(
            matches!(err, ErrorCode::Invalid),
            "unexpected error {err:?}"
        );
        dir.unlink_file_at("symlink").expect("removing a symlink");
    }

    // Clean up.
    drop(file);
    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_link(dir)
}
