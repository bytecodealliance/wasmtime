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

        let cases = [
            (
                Instant {
                    seconds: -1,
                    nanoseconds: 0,
                },
                Instant {
                    seconds: -1,
                    nanoseconds: 0,
                },
            ),
            (
                Instant {
                    seconds: -1,
                    nanoseconds: 500_000_000,
                },
                Instant {
                    seconds: -1,
                    nanoseconds: 500_000_000,
                },
            ),
            (
                Instant {
                    seconds: -1,
                    nanoseconds: 999_999_999,
                },
                Instant {
                    seconds: -1,
                    nanoseconds: 999_999_999,
                },
            ),
            (
                Instant {
                    seconds: -1,
                    nanoseconds: 1_500_000_000,
                },
                Instant {
                    seconds: 0,
                    nanoseconds: 500_000_000,
                },
            ),
        ];

        for (i, (timestamp, expected)) in cases.into_iter().enumerate() {
            let filename = format!("set-times-{i}.txt");
            let file = dir
                .open_at(
                    PathFlags::empty(),
                    filename,
                    OpenFlags::CREATE,
                    DescriptorFlags::READ | DescriptorFlags::WRITE,
                )
                .await
                .expect("creating a file");
            file.set_times(
                NewTimestamp::Timestamp(timestamp),
                NewTimestamp::Timestamp(timestamp),
            )
            .await
            .expect("setting the file's access timestamp");

            let stat = file.stat().await.expect("stating the file");
            let actual = stat
                .data_access_timestamp
                .expect("the file has an access timestamp");
            assert_eq!(actual.seconds, expected.seconds);
            assert_eq!(actual.nanoseconds, expected.nanoseconds);

            let filename = format!("set-times-at-{i}.txt");
            let file = dir
                .open_at(
                    PathFlags::empty(),
                    filename.clone(),
                    OpenFlags::CREATE,
                    DescriptorFlags::READ | DescriptorFlags::WRITE,
                )
                .await
                .expect("creating a file for set-times-at");
            dir.set_times_at(
                PathFlags::empty(),
                filename,
                NewTimestamp::Timestamp(timestamp),
                NewTimestamp::Timestamp(timestamp),
            )
            .await
            .expect("setting the file's access timestamp with set-times-at");

            let stat = file.stat().await.expect("stating the set-times-at file");
            let actual = stat
                .data_access_timestamp
                .expect("the set-times-at file has an access timestamp");
            assert_eq!(actual.seconds, expected.seconds);
            assert_eq!(actual.nanoseconds, expected.nanoseconds);
        }
        Ok(())
    }
}

fn main() {
    unreachable!()
}
