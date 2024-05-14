use test_programs::wasi;
use test_programs::wasi::filesystem::types::{DescriptorFlags, OpenFlags, PathFlags};

fn main() {
    let preopens = wasi::filesystem::preopens::get_directories();
    let (dir, _) = &preopens[0];

    let filename = "test.txt";
    let file = dir
        .open_at(
            PathFlags::empty(),
            filename,
            OpenFlags::CREATE,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .unwrap();
    let stream = file.write_via_stream(5).unwrap();
    stream.blocking_write_and_flush(b"Hello, ").unwrap();
    stream.blocking_write_and_flush(b"World!").unwrap();
    drop(stream);

    let stream = file.read_via_stream(0).unwrap();
    let contents = stream.blocking_read(100).unwrap();
    assert_eq!(contents, b"\0\0\0\0\0Hello, World!");
    drop(stream);

    // Test that file read streams behave like other read streams.
    let mut buf = Vec::new();
    let stream = file.read_via_stream(0).unwrap();
    let ready = stream.subscribe();
    loop {
        ready.block();

        match stream.read(0) {
            Ok(chunk) => assert!(chunk.is_empty()),
            Err(wasi::io::streams::StreamError::Closed) => break,
            Err(e) => panic!("Failed checking stream state: {e:?}"),
        }

        match stream.read(4) {
            Ok(chunk) => buf.extend(chunk),
            Err(wasi::io::streams::StreamError::Closed) => break,
            Err(e) => panic!("Failed reading stream: {e:?}"),
        }
    }
    assert_eq!(buf, b"\0\0\0\0\0Hello, World!");
    drop(ready);
    drop(stream);
    drop(file);

    dir.unlink_file_at(filename).unwrap();
}
