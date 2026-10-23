use std::time::Duration;
use wasip2::clocks::wall_clock::Datetime;
use wasip2::filesystem::preopens::get_directories;
use wasip2::filesystem::types::{Descriptor, DescriptorFlags, NewTimestamp, OpenFlags, PathFlags};

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
    // Create a file in the scratch directory.
    let file = dir
        .open_at(
            PathFlags::empty(),
            "file",
            OpenFlags::CREATE,
            DescriptorFlags::READ | DescriptorFlags::WRITE,
        )
        .expect("opening a file");

    // Check file size
    let file_stat = dir
        .stat_at(PathFlags::empty(), "file")
        .expect("reading file stats");
    assert_eq!(file_stat.size, 0, "file size should be 0");
    let file_mtim = to_duration(file_stat.data_modification_timestamp);

    // Create a symlink
    dir.symlink_at("file", "symlink")
        .expect("creating symlink to a file");

    // Check set_times_at on the symlink itself
    let sym_stat = dir
        .stat_at(PathFlags::empty(), "symlink")
        .expect("reading symlink stats");
    let sym_mtim = to_duration(sym_stat.data_modification_timestamp);

    // Modify mtim of symlink
    let sym_new_mtim = sym_mtim - Duration::from_secs(1);
    dir.set_times_at(
        PathFlags::empty(),
        "symlink",
        NewTimestamp::NoChange,
        to_timestamp(sym_new_mtim),
    )
    .expect("set_times_at should succeed on symlink");

    // Check that symlink mtim motification worked
    let modified_sym_stat = dir
        .stat_at(PathFlags::empty(), "symlink")
        .expect("reading file stats after set_times_at");

    assert_eq!(
        to_duration(modified_sym_stat.data_modification_timestamp),
        sym_new_mtim,
        "symlink mtim should change"
    );

    // Check that pointee mtim is not modified
    let unmodified_file_stat = dir
        .stat_at(PathFlags::empty(), "file")
        .expect("reading file stats after set_times_at");

    assert_eq!(
        to_duration(unmodified_file_stat.data_modification_timestamp),
        file_mtim,
        "file mtim should not change"
    );

    // Now, dereference the symlink
    let deref_sym_stat = dir
        .stat_at(PathFlags::SYMLINK_FOLLOW, "symlink")
        .expect("reading file stats on the dereferenced symlink");
    assert_eq!(
        to_duration(deref_sym_stat.data_modification_timestamp),
        file_mtim,
        "symlink mtim should be equal to pointee's when dereferenced"
    );

    // Finally, change stat of the original file by dereferencing the symlink
    dir.set_times_at(
        PathFlags::SYMLINK_FOLLOW,
        "symlink",
        NewTimestamp::NoChange,
        to_timestamp(sym_mtim),
    )
    .expect("set_times_at should succeed on setting stat on original file");

    let new_file_stat = dir
        .stat_at(PathFlags::empty(), "file")
        .expect("reading file stats after set_times_at");

    assert_eq!(
        to_duration(new_file_stat.data_modification_timestamp),
        sym_mtim,
        "mtim should change"
    );

    drop(file);
    dir.unlink_file_at("symlink").expect("removing a symlink");
    dir.unlink_file_at("file").expect("removing a file");
}

fn main() {
    let preopens = get_directories();
    let (dir, _name) = &preopens[0];

    // Run the tests.
    test_path_filestat(dir)
}
