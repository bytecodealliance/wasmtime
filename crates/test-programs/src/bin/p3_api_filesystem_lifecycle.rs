use futures::{future, join};
use test_programs::p3::wasi::filesystem::types::{DescriptorFlags, OpenFlags, PathFlags};
use test_programs::p3::{wasi, wit_stream};
use wit_bindgen::StreamResult;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let args = wasi::cli::environment::get_arguments();
        let scenario = args.get(2).expect("missing lifecycle scenario");
        let preopens = wasi::filesystem::preopens::get_directories();
        let (dir, _) = &preopens[0];
        if scenario == "readdir" {
            let (entries, status) = dir.read_directory();
            drop(status);
            entries.collect().await;
            return Ok(());
        }
        let file = dir
            .open_at(
                PathFlags::empty(),
                "lifecycle.txt".to_string(),
                OpenFlags::empty(),
                DescriptorFlags::READ | DescriptorFlags::WRITE,
            )
            .await
            .unwrap();

        if scenario == "write" || scenario == "append" {
            let (mut data_tx, data_rx) = wit_stream::new();
            join!(
                async {
                    if scenario == "write" {
                        file.write_via_stream(data_rx, 0).await.unwrap();
                    } else {
                        file.append_via_stream(data_rx).await.unwrap();
                    }
                },
                async {
                    assert!(data_tx.write_all(vec![b'x'; 16 * 1024]).await.is_empty());
                    drop(data_tx);
                },
            );
            return Ok(());
        }

        let offset = match scenario.as_str() {
            "read" | "eof" => 0,
            "error" => u64::MAX,
            other => panic!("unknown lifecycle scenario: {other}"),
        };
        let (mut stream, status) = file.read_via_stream(offset);
        // Closing the status reader must not release the active read session.
        let error_status = if scenario == "error" {
            Some(status)
        } else {
            drop(status);
            None
        };
        if scenario != "error" {
            for _ in 0..2 {
                let (result, buffer) = stream.read(Vec::with_capacity(8 * 1024)).await;
                assert!(matches!(result, StreamResult::Complete(_)));
                assert_eq!(buffer, vec![b'x'; 8 * 1024]);
            }
        }
        let (result, buffer) = stream.read(Vec::with_capacity(8 * 1024)).await;
        assert!(matches!(result, StreamResult::Dropped));
        assert!(buffer.is_empty());
        if let Some(status) = error_status {
            assert!(status.await.is_err(), "expected a native read error");
        }

        // This separate operation marks that the guest observed terminal EOF
        // or error. Keep the inert read stream alive after the marker completes.
        dir.stat().await.unwrap();
        future::pending::<()>().await;
        drop(stream);
        Ok(())
    }
}

fn main() {
    unreachable!()
}
