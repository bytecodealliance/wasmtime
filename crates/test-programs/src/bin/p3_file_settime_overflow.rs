use test_programs::p3::wasi::filesystem::types::{
    DescriptorFlags, ErrorCode, Instant, NewTimestamp, OpenFlags, PathFlags,
};
use test_programs::p3::{self, wasi};

struct Component;

p3::export!(Component);

impl p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = &preopens[0];

        let filename = "test.txt".to_owned();
        let file = dir
            .open_at(
                PathFlags::empty(),
                filename,
                OpenFlags::CREATE,
                DescriptorFlags::READ | DescriptorFlags::WRITE,
            )
            .await
            .unwrap();
        let err = file
            .set_times(
                NewTimestamp::Timestamp(Instant {
                    seconds: i64::MAX,
                    nanoseconds: 1_000_000_000,
                }),
                NewTimestamp::Timestamp(Instant {
                    seconds: 0,
                    nanoseconds: 0,
                }),
            )
            .await
            .err()
            .expect("expect set_times with overflowing timestamp to error");
        std::assert_matches!(
            err,
            ErrorCode::Overflow,
            "expect set_times error to be an overflow"
        );
        Ok(())
    }
}

fn main() {
    unreachable!()
}
