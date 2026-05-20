use test_programs::wasi::filesystem::preopens;
use test_programs::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};

const FILENAME: &str = "test.txt";
fn test_file_has_expected_contents(dir: &Descriptor) {
    // Open a file for reading
    let file = dir
        .open_at(
            PathFlags::empty(),
            FILENAME,
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .expect("open test.txt for reading");

    // Read the file's contents
    let stream = file.read_via_stream(0).unwrap();
    let read = stream.blocking_read(100).expect("reading test.txt content");
    drop(stream);
    drop(file);

    const EXPECTED_CONTENTS: &[u8] = b"truncation test file\n";
    // The file should not be empty due to truncation
    assert_eq!(read, EXPECTED_CONTENTS, "expected untouched file contents");
}

fn test_file_truncation_readonly(dir: &Descriptor) {
    // Check test preconditions.
    test_file_has_expected_contents(dir);

    // Opening the file for truncation should fail.
    let err = dir.open_at(
        PathFlags::empty(),
        FILENAME,
        OpenFlags::TRUNCATE,
        DescriptorFlags::READ,
    );
    assert!(err.is_err(), "opening file for truncation should fail");
    assert_eq!(
        err.err().unwrap(),
        ErrorCode::NotPermitted,
        "opening file for truncation should fail with ErrorCode::NotPermitted"
    );

    // Check that truncation did not occur.
    test_file_has_expected_contents(dir);
}

fn main() {
    // This test program requires a special preopen at the path "readonly",
    // which the host enforces as read-only. Unlike other test programs, this
    // directory's path not passed in as an argument, because modifications to
    // the testing harness would be too invasive.
    let preopens = preopens::get_directories();
    let (dir, _) = preopens
        .iter()
        .find(|(_, path)| path == "readonly")
        .expect("find preopen named readonly");

    // Run the test
    test_file_truncation_readonly(dir);
}
