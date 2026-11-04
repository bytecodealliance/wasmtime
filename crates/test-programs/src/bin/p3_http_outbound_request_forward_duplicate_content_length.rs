//! Forwards the headers of a response with two `content-length` values to a
//! new request, which must be rejected rather than sent.

use test_programs::p3::wasi::http::client;
use test_programs::p3::wasi::http::types::{ErrorCode, Fields, Method, Request, Scheme};
use test_programs::p3::wit_future;

struct Component;

test_programs::p3::export!(Component);

fn request(addr: &str, path: &str, headers: Fields) -> Request {
    let (_, trailers_rx) = wit_future::new(|| Ok(None));
    let (request, _transmit) = Request::new(headers, None, trailers_rx, None);
    request.set_method(&Method::Get).unwrap();
    request.set_scheme(Some(&Scheme::Http)).unwrap();
    request.set_authority(Some(addr)).unwrap();
    request.set_path_with_query(Some(path)).unwrap();
    request
}

/// Fetches `/dup`, whose response has two `content-length` values, and
/// returns its (immutable) headers.
async fn duplicate_headers(addr: &str) -> Fields {
    let response = client::send(request(addr, "/dup", Fields::new()))
        .await
        .unwrap();
    let headers = response.get_headers();
    assert_eq!(
        headers.get("content-length"),
        [b"5".to_vec(), b"100".to_vec()],
        "precondition: the response must carry both values"
    );
    headers
}

async fn assert_rejected(addr: &str, headers: Fields) {
    let err = client::send(request(addr, "/victim", headers))
        .await
        .err()
        .expect("request with multiple `content-length` values must be rejected");
    assert!(
        matches!(&err, ErrorCode::InternalError(Some(msg)) if msg.contains("content-length")),
        "unexpected error: {err:?}"
    );
}

impl test_programs::p3::exports::wasi::cli::run::Guest for Component {
    async fn run() -> Result<(), ()> {
        let addr = test_programs::p3::wasi::cli::environment::get_environment()
            .into_iter()
            .find_map(|(k, v)| k.eq("HTTP_SERVER").then_some(v))
            .unwrap();

        println!("forwarding response headers");
        assert_rejected(&addr, duplicate_headers(&addr).await).await;

        println!("forwarding cloned response headers");
        assert_rejected(&addr, duplicate_headers(&addr).await.clone()).await;

        Ok(())
    }
}

fn main() {}
