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

        wit_bindgen::spawn(async move {
            let remaining = body_tx.write_all(b"Hello, WASI!".to_vec()).await;
            assert!(remaining.is_empty());
        });
        Ok(response)
    }
}

fn main() {
    unreachable!()
}
