use test_programs::p3::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};
use test_programs::p3::{self, wasi};

struct Component;

p3::export!(Component);

const RW_ALIAS_FILENAME: &str = "alias.txt";
const RO_TEST_FILENAME: &str = "test.txt";
const RO_EXPECTED_CONTENTS: &[u8] = b"read only test file\n";

async fn test_ro_file_has_expected_contents(dir: &Descriptor) {
    // Open a file for reading
    let file = dir
        .open_at(
            PathFlags::empty(),
            RO_TEST_FILENAME.to_string(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .expect("open test.txt for reading");

    // Read the file's contents
    let (read, result) = file.read_via_stream(0);
    let read = read.collect().await;
    result.await.expect("reading test.txt content");
    drop(file);

    assert_eq!(
        read, RO_EXPECTED_CONTENTS,
        "expected untouched file contents"
    );
}

async fn test_file_hardlink_across_perms(rw_dir: &Descriptor, ro_dir: &Descriptor) {
    // Check test preconditions.
    test_ro_file_has_expected_contents(ro_dir).await;

    // Creating a hard link of the read-only file into a Descriptor under
    // which files are read-writable would allow the read-only file to be
    // written to. So, this must fail with perm:
    let err = ro_dir
        .link_at(
            PathFlags::empty(),
            RO_TEST_FILENAME.to_string(),
            rw_dir,
            RW_ALIAS_FILENAME.to_string(),
        )
        .await;
    assert!(
        err.is_err(),
        "link_at should fail because link source is readonly, dest is readwrite"
    );
    assert!(
        matches!(err.err().unwrap(), ErrorCode::NotPermitted,),
        "link_at should fail with NotPermitted"
    );

    // Check that contents of link dest did not change
    test_ro_file_has_expected_contents(ro_dir).await;
}

impl p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let args = wasi::cli::environment::get_arguments();
        if args.len() != 2 {
            panic!("usage: scratch directory argument required");
        }
        let preopens = wasi::filesystem::preopens::get_directories();
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
        test_file_hardlink_across_perms(rw_dir, ro_dir).await;

        Ok(())
    }
}

fn main() {
    unreachable!()
}
