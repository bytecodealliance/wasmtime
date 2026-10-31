//! Checks that a response with ambiguous framing is rejected.

use test_programs::p3::wasi::http::client;
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Method, Request, Scheme};
use test_programs::p3::wit_future;

struct Component;

test_programs::p3::export!(Component);

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();
        let (_, trailers_rx) = wit_future::new(|| Ok(None));
        let (request, _transmit) = Request::new(Fields::new(), None, trailers_rx, None);
        request.set_method(&Method::Get).unwrap();
        request.set_scheme(Some(&Scheme::Http)).unwrap();
        request.set_authority(Some(&addr)).unwrap();
        request.set_path_with_query(Some("/")).unwrap();

        let err = client::send(request)
            .await
            .err()
            .expect("ambiguous response must be rejected");
        assert!(
            matches!(&err, ErrorCode::HttpProtocolError),
            "unexpected error: {err:?}"
        );
        println!("rejected ambiguous response");
        Ok(())
    }
}

fn main() {}
