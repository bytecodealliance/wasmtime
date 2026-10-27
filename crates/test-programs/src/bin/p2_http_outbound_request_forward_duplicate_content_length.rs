//! Forwards the headers of a response with two `content-length` values to a
//! new request, which must be rejected rather than sent.

use test_programs::wasi::http::outgoing_handler;
use test_programs::wasi::http::types::{ErrorCode, Fields, Method, OutgoingRequest, Scheme};

fn request(addr: &str, path: &str, headers: Fields) -> OutgoingRequest {
    let request = OutgoingRequest::new(headers);
    request.set_method(&Method::Get).unwrap();
    request.set_scheme(Some(&Scheme::Http)).unwrap();
    request.set_authority(Some(addr)).unwrap();
    request.set_path_with_query(Some(path)).unwrap();
    request
}

/// Fetches `/dup`, whose response has two `content-length` values, and
/// returns its (immutable) headers.
fn duplicate_headers(addr: &str) -> Fields {
    let future = outgoing_handler::handle(request(addr, "/dup", Fields::new()), None).unwrap();
    future.subscribe().block();
    let response = future.get().unwrap().unwrap().unwrap();
    let headers = response.headers();
    assert_eq!(
        headers.get("content-length"),
        [b"5".to_vec(), b"100".to_vec()],
        "precondition: the response must carry both values"
    );
    headers
}

fn assert_rejected(addr: &str, headers: Fields) {
    let err = outgoing_handler::handle(request(addr, "/victim", headers), None)
        .err()
        .expect("request with multiple `content-length` values must be rejected");
    assert!(
        matches!(&err, ErrorCode::InternalError(Some(msg)) if msg.contains("content-length")),
        "unexpected error: {err:?}"
    );
}

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();

    println!("forwarding response headers");
    assert_rejected(&addr, duplicate_headers(&addr));

    println!("forwarding cloned response headers");
    assert_rejected(&addr, duplicate_headers(&addr).clone());
}
