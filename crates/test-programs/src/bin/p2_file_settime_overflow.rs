use test_programs::wasi::filesystem::preopens;
use test_programs::wasi::filesystem::types::{
    Datetime, DescriptorFlags, ErrorCode, NewTimestamp, OpenFlags, PathFlags,
};
fn main() {
    let preopens = preopens::get_directories();
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
    let err = file
        .set_times(
            NewTimestamp::Timestamp(Datetime {
                seconds: u64::MAX,
                nanoseconds: 1_000_000_000,
            }),
            NewTimestamp::Timestamp(Datetime {
                seconds: 0,
                nanoseconds: 0,
            }),
        )
        .err()
        .expect("expect set_times with overflowing timestamp to error");
    assert_eq!(
        err,
        ErrorCode::Overflow,
        "expect set_times error to be an overflow"
    );
}
