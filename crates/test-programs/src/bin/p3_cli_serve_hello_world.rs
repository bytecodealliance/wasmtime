use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response};
use test_programs::p3::{service, wit_future, wit_stream};

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let (mut body_tx, body_rx) = wit_stream::new();
        let (body_result_tx, body_result_rx) = wit_future::new(|| Ok(None));
        let (response, _future_result) =
            Response::new(Fields::new(), Some(body_rx), body_result_rx);
        let trap = request.get_path_with_query().as_deref() == Some("/trap");

        wit_bindgen::spawn_local(async move {
            let remaining = body_tx.write_all(b"Hello, WASI!".to_vec()).await;
            assert!(remaining.is_empty());
            if trap {
                unreachable!();
            }
            drop(body_result_tx);
        });
        Ok(response)
    }
}

fn main() {
    unreachable!()
}
