use test_programs::p3::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};
use test_programs::p3::{self, wasi};

struct Component;

p3::export!(Component);

const FILENAME: &str = "test.txt";
const EXPECTED_CONTENTS: &[u8] = b"truncation test file\n";

impl p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = preopens
            .iter()
            .find(|(_, path)| path == "readonly")
            .expect("find preopen named readonly");

        test_file_truncation_readonly(dir).await;
        Ok(())
    }
}

fn main() {
    unreachable!()
}

async fn test_file_has_expected_contents(dir: &Descriptor) {
    let file = dir
        .open_at(
            PathFlags::empty(),
            FILENAME.to_string(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .expect("open test.txt for reading");

    let (read, result) = file.read_via_stream(0);
    let read = read.collect().await;
    result.await.expect("reading test.txt content");
    drop(file);

    assert_eq!(read, EXPECTED_CONTENTS, "expected untouched file contents");
}

async fn test_file_truncation_readonly(dir: &Descriptor) {
    test_file_has_expected_contents(dir).await;

    let err = dir
        .open_at(
            PathFlags::empty(),
            FILENAME.to_string(),
            OpenFlags::TRUNCATE,
            DescriptorFlags::READ,
        )
        .await
        .expect_err("opening file for truncation should fail");
    assert!(
        matches!(err, ErrorCode::NotPermitted),
        "opening file for truncation should fail with ErrorCode::NotPermitted, got {err:?}"
    );

    test_file_has_expected_contents(dir).await;
}
