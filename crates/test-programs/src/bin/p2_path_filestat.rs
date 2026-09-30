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

fn test_path_filestat(dir: &Descriptor) {
    let flags = DescriptorFlags::READ | DescriptorFlags::WRITE;

    // Create a file in the scratch directory.
    let file = dir
        .open_at(
            PathFlags::empty(),
            "file",
            OpenFlags::CREATE,
            // Pass some flags for later retrieval
            flags,
        )
        .expect("opening a file");

    let file_flags = file.get_flags().expect("get_flags");
    assert!(
        file_flags.contains(flags),
        "file should have the flags used to create the file"
    );
    let err = dir
        .open_at(
            PathFlags::empty(),
            "file",
            OpenFlags::empty(),
            flags | DescriptorFlags::FILE_INTEGRITY_SYNC,
        )
        .expect_err("FILE_INTEGRITY_SYNC not supported by platform");
    assert!(
        matches!(err, ErrorCode::Unsupported),
        "unexpected error {err:?}"
    );

    // Check file size
    let file_stat = dir
        .stat_at(PathFlags::empty(), "file")
        .expect("reading file stats");
    assert_eq!(file_stat.size, 0, "file size should be 0");

    // Check set_times_at
    let new_mtim = to_duration(file_stat.data_modification_timestamp) - Duration::from_secs(1);
    dir.set_times_at(
        PathFlags::empty(),
        "file",
        NewTimestamp::NoChange,
        to_timestamp(new_mtim),
    )
    .expect("set_times_at should succeed");

    let modified_file_stat = dir
        .stat_at(PathFlags::empty(), "file")
        .expect("reading file stats after set_times_at");

    assert_eq!(
        to_duration(modified_file_stat.data_modification_timestamp),
        new_mtim,
        "mtim should change"
    );

    drop(file);
    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_filestat(dir)
}
