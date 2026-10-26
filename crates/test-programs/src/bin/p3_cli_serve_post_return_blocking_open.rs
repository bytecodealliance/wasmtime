use test_programs::p3::wasi::filesystem::types::{DescriptorFlags, OpenFlags, PathFlags};
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response};
use test_programs::p3::{service, wasi, wit_future, wit_stream};

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        let (mut body_tx, body_rx) = wit_stream::new();
        let (body_result_tx, body_result_rx) = wit_future::new(|| Ok(None));
        let (response, _future_result) =
            Response::new(Fields::new(), Some(body_rx), body_result_rx);
        drop(body_result_tx);

        // After responding, open a FIFO for reading, which blocks until the
        // host test opens the other end for writing.
        wit_bindgen::spawn_local(async move {
            let preopens = wasi::filesystem::preopens::get_directories();
            let (dir, _) = &preopens[0];
            dir.open_at(
                PathFlags::empty(),
                "fifo".to_string(),
                OpenFlags::empty(),
                DescriptorFlags::READ,
            )
            .await
            .expect("opening the fifo");
            let remaining = body_tx.write_all(b"done".to_vec()).await;
            assert!(remaining.is_empty());
        });
        Ok(response)
    }
}

fn main() {
    unreachable!()
}
