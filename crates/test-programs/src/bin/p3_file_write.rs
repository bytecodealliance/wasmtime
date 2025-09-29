use futures::join;
use test_programs::p3::wasi::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags,
};
use test_programs::p3::{wasi, wit_stream};

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = &preopens[0];

        test_file_long_write(dir, "long_write.txt").await;
        Ok(())
    }
}

fn main() {
    unreachable!()
}

async fn test_file_long_write(dir: &Descriptor, filename: &str) {
    let mut content = Vec::new();
    // 16 byte string, 4096 times, is 64k
    for n in 0..4096 {
        let chunk = format!("123456789 {n:05} ");
        assert_eq!(chunk.as_str().as_bytes().len(), 16);
        content.extend_from_slice(chunk.as_str().as_bytes());
    }

    // Write to the file
    let file = dir
        .open_at(
            PathFlags::empty(),
            filename.to_string(),
            OpenFlags::CREATE,
            DescriptorFlags::WRITE,
        )
        .await
        .expect("creating a file for writing");
    let (mut tx, rx) = wit_stream::new();
    join! {
        async {
            file.write_via_stream(rx, 0).await.unwrap();
        },
        async {
            let result = tx.write_all(content.clone()).await;
            drop(tx);
            assert!(result.is_empty());
        },
    };

    // The file should be of the appropriate size via `stat` now.
    let stat = file.stat().await.unwrap();
    assert_eq!(
        stat.size,
        content.len() as u64,
        "file should be size of content",
    );

    drop(file);

    // Make sure the file can be read at various offsets.
    let file = dir
        .open_at(
            PathFlags::empty(),
            filename.to_string(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .expect("creating a file for reading");

    let (read_contents, result) = file.read_via_stream(0);
    let read_contents = read_contents.collect().await;
    result.await.unwrap();
    assert!(read_contents == content);

    let (read_contents, result) = file.read_via_stream((content.len() as u64) - 100);
    let read_contents = read_contents.collect().await;
    result.await.unwrap();
    assert!(read_contents == &content[content.len() - 100..]);
    drop(file);

    // Writing to a read-only handle should be an error.
    let filename = "test-zero-write-fails.txt";
    dir.open_at(
        PathFlags::empty(),
        filename.to_string(),
        OpenFlags::CREATE,
        DescriptorFlags::WRITE,
    )
    .await
    .expect("creating a file for writing");
    let file = dir
        .open_at(
            PathFlags::empty(),
            filename.to_string(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
        )
        .await
        .expect("creating a file for writing");

    let (mut tx, rx) = wit_stream::new();
    join! {
        async {
            let err = file.write_via_stream(rx, 0).await.unwrap_err();
            assert!(
                matches!(err, ErrorCode::Access | ErrorCode::BadDescriptor | ErrorCode::NotPermitted),
                "bad error {err:?}",
            );
        },
        async {
            let result = tx.write_all(b"x".to_vec()).await;
            drop(tx);
            assert_eq!(result.len(), 1);
        },
    };
}
