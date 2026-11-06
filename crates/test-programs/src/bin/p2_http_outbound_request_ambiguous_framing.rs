//! Checks that a response with ambiguous framing is rejected.

use test_programs::wasi::http::outgoing_handler;
use test_programs::wasi::http::types::{ErrorCode, Fields, Method, OutgoingRequest, Scheme};

fn main() {
    let addr = std::env::var("HTTP_SERVER").unwrap();

    let request = OutgoingRequest::new(Fields::new());
    request.set_method(&Method::Get).unwrap();
    request.set_scheme(Some(&Scheme::Http)).unwrap();
    request.set_authority(Some(&addr)).unwrap();
    request.set_path_with_query(Some("/")).unwrap();

    let future = outgoing_handler::handle(request, None).unwrap();
    future.subscribe().block();
    let err = match future.get() {
        Some(Ok(Err(e))) => e,
        Some(Ok(Ok(_))) => panic!("ambiguous response must be rejected"),
        Some(Err(())) => panic!("unexpected outer error"),
        None => panic!("future unexpectedly pending"),
    };
    assert!(
        matches!(&err, ErrorCode::HttpProtocolError),
        "unexpected error: {err:?}"
    );
    println!("rejected ambiguous response");
}
