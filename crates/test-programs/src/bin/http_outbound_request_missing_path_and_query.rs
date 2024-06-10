use test_programs::wasi::http::outgoing_handler::{handle, OutgoingRequest};
use test_programs::wasi::http::types::{Fields, Method, Scheme};

fn main() {
    let fields = Fields::new();
    let req = OutgoingRequest::new(fields);
    req.set_method(&Method::Get).unwrap();
    req.set_scheme(Some(&Scheme::Https)).unwrap();
    req.set_authority(Some("example.com")).unwrap();

    // Don't set path/query
    // req.set_path_with_query(Some("/")).unwrap();

    let res = handle(req, None);
    assert!(res.is_err());
}
