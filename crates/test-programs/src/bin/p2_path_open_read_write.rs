use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, OpenFlags, PathFlags};
use wasip2::io::streams::StreamError;

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

fn open(dir: &Descriptor, path: &str, flags: DescriptorFlags) -> Descriptor {
    dir.open_at(PathFlags::empty(), path, OpenFlags::empty(), flags)
        .expect("opening a file")
}

/// Reads the entire contents of `file` starting at `offset`.
fn read(file: &Descriptor, offset: u64) -> Result<Vec<u8>, StreamError> {
    let stream = file.read_via_stream(offset).expect("read_via_stream");
    let mut buf = Vec::new();
    loop {
        match stream.blocking_read(100) {
            Ok(chunk) => buf.extend(chunk),
            Err(StreamError::Closed) => return Ok(buf),
            Err(e) => return Err(e),
        }
    }
}

/// Writes all of `bytes` to `file` at `offset`.
fn write(file: &Descriptor, offset: u64, bytes: &[u8]) -> Result<(), StreamError> {
    let stream = file.write_via_stream(offset).expect("write_via_stream");
    stream.blocking_write_and_flush(bytes)
}

fn test_path_open_read_write(dir: &Descriptor) {
    create_file(dir, "file");

    let f_readonly = open(dir, "file", DescriptorFlags::READ);

    let flags = f_readonly.get_flags().expect("get flags readonly");
    assert!(
        flags.contains(DescriptorFlags::READ),
        "readonly has read flag"
    );
    assert!(
        !flags.contains(DescriptorFlags::WRITE),
        "readonly does not have write flag"
    );

    let contents = read(&f_readonly, 0).expect("reading readonly file");
    assert_eq!(contents.len(), 0, "readonly file is empty");

    let write_buffer = &[1u8; 50];
    let err = write(&f_readonly, 0, write_buffer)
        .err()
        .expect("write of readonly fails");
    assert!(
        matches!(err, StreamError::LastOperationFailed(_)),
        "unexpected error {err:?}"
    );

    drop(f_readonly);

    // =============== WRITE ONLY ==================
    let f_writeonly = open(dir, "file", DescriptorFlags::WRITE);

    let flags = f_writeonly.get_flags().expect("get flags writeonly");
    assert!(
        !flags.contains(DescriptorFlags::READ),
        "writeonly does not have read flag"
    );
    assert!(
        flags.contains(DescriptorFlags::WRITE),
        "writeonly has write flag"
    );

    let err = read(&f_writeonly, 0)
        .err()
        .expect("read of writeonly fails");
    assert!(
        matches!(err, StreamError::LastOperationFailed(_)),
        "unexpected error {err:?}"
    );
    write(&f_writeonly, 0, write_buffer).expect("write to writeonly");

    drop(f_writeonly);

    // ============== READ WRITE =======================

    let f_readwrite = open(dir, "file", DescriptorFlags::READ | DescriptorFlags::WRITE);
    let flags = f_readwrite.get_flags().expect("get flags readwrite");
    assert!(
        flags.contains(DescriptorFlags::READ),
        "readwrite has read flag"
    );
    assert!(
        flags.contains(DescriptorFlags::WRITE),
        "readwrite has write flag"
    );

    let contents = read(&f_readwrite, 0).expect("reading readwrite file");
    assert_eq!(
        contents.len(),
        write_buffer.len(),
        "readwrite file contains contents from writeonly open"
    );

    let write_buffer_2 = &[2u8; 25];
    write(&f_readwrite, write_buffer.len() as u64, write_buffer_2).expect("write to readwrite");

    let stat = f_readwrite.stat().expect("get stat readwrite");
    assert_eq!(
        stat.size as usize,
        write_buffer.len() + write_buffer_2.len(),
        "total written is both write buffers"
    );

    drop(f_readwrite);

    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_open_read_write(dir)
}
