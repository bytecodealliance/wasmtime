use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{DescriptorFlags, DescriptorType, ErrorCode, OpenFlags, PathFlags};

fn path_open_preopen() {
    let preopens = get_directories();
    let (preopen, _name) = &preopens[0];

    assert_eq!(
        preopen.get_type().expect("get type"),
        DescriptorType::Directory,
        "preopen is a directory"
    );

    // Hard-code the set of flags expected for a preopened directory. Various
    // userland implementations expect (at least) this set of flags to be
    // present on all directories:
    let flags = preopen.get_flags().expect("get flags");
    for (flag, name) in [
        (DescriptorFlags::READ, "READ"),
        (DescriptorFlags::MUTATE_DIRECTORY, "MUTATE_DIRECTORY"),
    ] {
        assert!(
            flags.contains(flag),
            "flags do not have required flag `{name}`"
        );
    }

    // Open with same flags it has now:
    let _ = preopen
        .open_at(PathFlags::empty(), ".", OpenFlags::empty(), flags)
        .expect("open with same flags");

    // Open with an empty set of flags:
    let _ = preopen
        .open_at(
            PathFlags::empty(),
            ".",
            OpenFlags::empty(),
            DescriptorFlags::empty(),
        )
        .expect("open with empty flags");

    // Open DIRECTORY with an empty set of flags:
    let _ = preopen
        .open_at(
            PathFlags::empty(),
            ".",
            OpenFlags::DIRECTORY,
            DescriptorFlags::empty(),
        )
        .expect("open with DIRECTORY empty flags");

    // Open DIRECTORY with just the read flag:
    let _ = preopen
        .open_at(
            PathFlags::empty(),
            ".",
            OpenFlags::DIRECTORY,
            DescriptorFlags::READ,
        )
        .expect("open with DIRECTORY and read flag");

    // Open DIRECTORY and read/write flags should fail with is-directory:
    let err = preopen
        .open_at(
            PathFlags::empty(),
            ".",
            OpenFlags::DIRECTORY,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .expect_err("open with DIRECTORY and read/write should fail");
    assert!(
        matches!(err, ErrorCode::IsDirectory),
        "opening directory read/write should fail with is-directory, got {err:?}"
    );
}

fn main() {
    path_open_preopen();
}
