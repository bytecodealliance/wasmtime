//! A request with live `headers` children can still be handed to the handler.

use test_programs::wasi::http::outgoing_handler;
use test_programs::wasi::http::types::{Fields, Method, OutgoingRequest, Scheme};

fn main() {
    let address = std::env::var("HTTP_SERVER").unwrap();
    let request = OutgoingRequest::new(Fields::new());
    request.set_method(&Method::Get).unwrap();
    request.set_scheme(Some(&Scheme::Http)).unwrap();
    request.set_authority(Some(&address)).unwrap();
    request.set_path_with_query(Some("/get")).unwrap();

    // Enough calls to exercise the bookkeeping prune path.
    for _ in 0..100 {
        let headers = request.headers();
        drop(headers);
    }

    let headers = request.headers();
    let future = outgoing_handler::handle(request, None).unwrap();
    future.subscribe().block();
    let response = future.get().unwrap().unwrap().unwrap();
    assert_eq!(response.status(), 200);
    drop(headers);
}
