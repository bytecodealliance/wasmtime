#![expect(unsafe_op_in_unsafe_fn, reason = "old code, not worth updating yet")]
use test_programs::wasi::filesystem::preopens;
use test_programs::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};

const RW_ALIAS_FILENAME: &str = "alias.txt";
const RO_TEST_FILENAME: &str = "test.txt";
const RO_EXPECTED_CONTENTS: &[u8] = b"read only test file\n";

unsafe fn test_ro_file_has_expected_contents(dir: &Descriptor) {
    // Open a file for reading
    let file = dir
        .open_at(
            PathFlags::empty(),
            RO_TEST_FILENAME,
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .expect("open test.txt for reading");

    // Read the file's contents
    let stream = file.read_via_stream(0).unwrap();
    let read = stream.blocking_read(100).expect("reading test.txt content");

    drop(stream);
    drop(file);
    assert_eq!(
        read, RO_EXPECTED_CONTENTS,
        "expected untouched file contents"
    );
}

unsafe fn test_file_hardlink_across_perms(rw_dir: &Descriptor, ro_dir: &Descriptor) {
    // Check test preconditions.
    test_ro_file_has_expected_contents(ro_dir);

    // Creating a hard link of the read-only file into a Descriptor under
    // which files are read-writable would allow the read-only file to be
    // written to. So, this must fail with perm:
    let err = ro_dir.link_at(
        PathFlags::empty(),
        RO_TEST_FILENAME,
        rw_dir,
        RW_ALIAS_FILENAME,
    );
    assert!(
        err.is_err(),
        "link_at should fail because link source is readonly, and dest is readwrite"
    );
    assert_eq!(
        err.err().unwrap(),
        ErrorCode::NotPermitted,
        "link_at should fail with NotPermitted"
    );

    // Check that contents of link dest did not change
    test_ro_file_has_expected_contents(ro_dir);
}

fn main() {
    let args = wasip2::cli::environment::get_arguments();
    if args.len() != 2 {
        panic!("usage: scratch directory argument required");
    }
    let preopens = preopens::get_directories();
    let rw_path = &args[1];
    let (rw_dir, _) = preopens
        .iter()
        .find(|(_, path)| path == rw_path)
        .expect("find preopen specified by argument");

    // This test program requires a special preopen at the path "readonly",
    // which the host enforces as read-only. Unlike other test programs, this
    // directory's path not passed in as an argument, because modifications to
    // the testing harness would be too invasive.
    let (ro_dir, _) = preopens
        .iter()
        .find(|(_, path)| path == "readonly")
        .expect("find preopen named readonly");

    // Run the tests.
    unsafe {
        test_file_hardlink_across_perms(rw_dir, ro_dir);
    }
}
