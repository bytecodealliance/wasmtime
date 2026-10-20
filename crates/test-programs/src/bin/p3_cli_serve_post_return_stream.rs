use test_programs::p3::wasi::clocks::monotonic_clock;
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response};
use test_programs::p3::{service, wit_future, wit_stream};

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        let (mut body_tx, body_rx) = wit_stream::new();
        let (body_result_tx, body_result_rx) = wit_future::new(|| Ok(None));
        let (response, _future_result) =
            Response::new(Fields::new(), Some(body_rx), body_result_rx);
        drop(body_result_tx);

        // Keep writing the body for ~3 seconds after the response is returned.
        wit_bindgen::spawn_local(async move {
            for chunk in ["1", "2", "3"] {
                let remaining = body_tx.write_all(chunk.as_bytes().to_vec()).await;
                assert!(remaining.is_empty());
                monotonic_clock::wait_for(1_000_000_000 /* ns */).await;
            }
        });
        Ok(response)
    }
}

fn main() {
    unreachable!()
}
