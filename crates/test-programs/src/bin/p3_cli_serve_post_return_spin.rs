use test_programs::p3::wasi::clocks::monotonic_clock;
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response};
use test_programs::p3::{service, wit_future, wit_stream};

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        let (body_tx, body_rx) = wit_stream::new();
        let (body_result_tx, body_result_rx) = wit_future::new(|| Ok(None));
        let (response, _future_result) =
            Response::new(Fields::new(), Some(body_rx), body_result_rx);
        drop(body_result_tx);

        // After the response has been delivered, spin forever without
        // yielding, keeping the body open so it only ends when the instance
        // is dropped.
        wit_bindgen::spawn_local(async move {
            let _body_tx = body_tx;
            monotonic_clock::wait_for(100_000_000 /* ns */).await;
            loop {
                std::hint::spin_loop();
            }
        });
        Ok(response)
    }
}

fn main() {
    unreachable!()
}
