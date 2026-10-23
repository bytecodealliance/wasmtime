//! Checks that `serve` fills in the scheme and authority of an incoming request.
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Request, Response, Scheme};
use test_programs::p3::{service, wit_future};

struct T;

service::export!(T);

impl service::exports::wasi::http::handler::Guest for T {
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        assert_eq!(
            request.get_authority().as_deref(),
            Some("localhost"),
            "unexpected authority"
        );
        assert!(
            matches!(request.get_scheme(), Some(Scheme::Http)),
            "unexpected scheme: {:?}",
            request.get_scheme()
        );

        let (body_result_tx, body_result_rx) = wit_future::new(|| Ok(None));
        let (response, _result) = Response::new(Fields::new(), None, body_result_rx);
        drop(body_result_tx);
        Ok(response)
    }
}

fn main() {
    unreachable!()
}
