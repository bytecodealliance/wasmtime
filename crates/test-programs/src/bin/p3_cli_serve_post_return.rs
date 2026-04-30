use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response};
use test_programs::p3::{service, wit_future};

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(_request: Request) -> Result<Response, ErrorCode> {
        let (body_result_tx, body_result_rx) = wit_future::new(|| Ok(None));
        let (response, _future_result) = Response::new(Fields::new(), None, body_result_rx);
        drop(body_result_tx);

        wit_bindgen::spawn(async move {
            for _ in 0..10 {
                wit_bindgen::yield_async().await;
            }
            println!("please see me");
        });
        Ok(response)
    }
}

fn main() {
    unreachable!()
}
