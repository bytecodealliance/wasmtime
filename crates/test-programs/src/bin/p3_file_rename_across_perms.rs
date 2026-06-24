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

async fn test_file_rename_across_perms(rw_dir: &Descriptor, ro_dir: &Descriptor) {
    // Check test preconditions.
    test_ro_file_has_expected_contents(ro_dir).await;

    // Create a hardlink inside the file ro dir so there are two files pointing to
    // the read-only file.
    ro_dir
        .link_at(
            PathFlags::empty(),
            RO_TEST_FILENAME.to_owned(),
            ro_dir,
            RW_ALIAS_FILENAME.to_owned(),
        )
        .await
        .expect("should be possible to create link inside ro file domain");

    // Renaming that file into the file rw dir should fail with permissions
    // error, otherwise it would permit opening the ro file as rw
    let err = ro_dir
        .rename_at(
            RW_ALIAS_FILENAME.to_owned(),
            rw_dir,
            RW_ALIAS_FILENAME.to_owned(),
        )
        .await;
    assert!(
        err.is_err(),
        "rename_at should fail because link source is file readonly, and dest is file readwrite"
    );
    assert!(
        matches!(err.err().unwrap(), ErrorCode::NotPermitted,),
        "rename_at should fail with NotPermitted"
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
        test_file_rename_across_perms(rw_dir, ro_dir).await;

        Ok(())
    }
}

fn main() {
    unreachable!()
}
