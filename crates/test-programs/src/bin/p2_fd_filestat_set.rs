use std::time::Duration;
use wasip2::clocks::wall_clock::Datetime;
use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{
    Descriptor, DescriptorFlags, ErrorCode, NewTimestamp, OpenFlags, PathFlags,
};

fn to_duration(t: Option<Datetime>) -> Duration {
    let t = t.expect("timestamp should be present");
    Duration::new(t.seconds, t.nanoseconds)
}

fn to_timestamp(d: Duration) -> NewTimestamp {
    NewTimestamp::Timestamp(Datetime {
        seconds: d.as_secs(),
        nanoseconds: d.subsec_nanos(),
    })
}

fn create_file(dir: &Descriptor, path: &str) {
    let file = dir
        .open_at(
            PathFlags::empty(),
            path,
            OpenFlags::CREATE,
            DescriptorFlags::empty(),
        )
        .expect("failed to create file");
    drop(file);
}

fn open_file(dir: &Descriptor, path: &str, flags: DescriptorFlags) -> Descriptor {
    dir.open_at(PathFlags::empty(), path, OpenFlags::empty(), flags)
        .expect("failed to open file")
}

fn test_fd_filestat_set_size_rw(dir: &Descriptor) {
    // Create a file in the scratch directory, opened read/write
    let file = dir
        .open_at(
            PathFlags::empty(),
            "file",
            OpenFlags::CREATE,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .expect("failed to create file");

    // Check file size
    let stat = file.stat().expect("failed stat");
    assert_eq!(stat.size, 0, "file size should be 0");

    // Check set_size
    file.set_size(100).expect("set_size");

    let stat = file.stat().expect("failed stat 2");
    assert_eq!(stat.size, 100, "file size should be 100");

    drop(file);
    dir.unlink_file_at("file").expect("failed to remove file");
}

fn test_fd_filestat_set_size_ro(dir: &Descriptor) {
    // Create a file in the scratch directory. Creating a file implies opening it for writing, so
    // we have to close and re-open read-only to observe read-only behavior.
    create_file(dir, "file");

    // Open the created file read-only
    let file = open_file(dir, "file", DescriptorFlags::READ);

    // Check file size
    let stat = file.stat().expect("failed stat");
    assert_eq!(stat.size, 0, "file size should be 0");

    // Check set_size on a file opened read-only fails with `invalid`, like
    // ftruncate is defined to do on posix (or `access` on Windows).
    let err = file
        .set_size(100)
        .expect_err("set_size should error when file is opened read-only");
    assert!(
        matches!(err, ErrorCode::Invalid | ErrorCode::Access),
        "unexpected error {err:?}"
    );

    let stat = file.stat().expect("failed stat 2");
    assert_eq!(stat.size, 0, "file size should remain 0");

    drop(file);
    dir.unlink_file_at("file").expect("failed to remove file");
}

fn test_fd_filestat_set_times(dir: &Descriptor, flags: DescriptorFlags) {
    // Create a file in the scratch directory. CREATE implies opening for writing, so we will
    // close it and re-open with the desired flags (READ for read only, READ | WRITE for
    // readwrite)
    create_file(dir, "file");

    // Open the file with the flags given.
    let file = open_file(dir, "file", flags);

    let stat = file.stat().expect("failed stat 2");

    // Check set_times
    let old_atim = to_duration(stat.data_access_timestamp);
    let new_mtim = to_duration(stat.data_modification_timestamp) - Duration::from_secs(1);
    let result = file.set_times(NewTimestamp::NoChange, to_timestamp(new_mtim));
    if !flags.contains(DescriptorFlags::WRITE) && result.is_err() {
        // Windows rejects set-times on read-only files, so skip the rest of
        // this test in that case.
        drop(file);
        dir.unlink_file_at("file").expect("failed to remove file");
        return;
    }
    result.expect("set_times");

    let stat = file.stat().expect("failed stat 3");
    assert_eq!(stat.size, 0, "file size should remain unchanged at 0");

    assert_eq!(
        to_duration(stat.data_modification_timestamp),
        new_mtim,
        "mtim should change"
    );
    assert_eq!(
        to_duration(stat.data_access_timestamp),
        old_atim,
        "atim should not change"
    );

    drop(file);
    dir.unlink_file_at("file").expect("failed to remove file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_fd_filestat_set_size_rw(dir);
    test_fd_filestat_set_size_ro(dir);

    // The set_times function should behave the same whether the file
    // descriptor is open for read-only or read-write, because the underlying
    // permissions of the file determine whether or not the filestat can be
    // set or not, not than the open mode.
    test_fd_filestat_set_times(dir, DescriptorFlags::READ);
    test_fd_filestat_set_times(dir, DescriptorFlags::READ | DescriptorFlags::WRITE);
}
