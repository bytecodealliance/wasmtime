//! Prints the number of bytes a single write to an outgoing request body may
//! carry, which is what the `--http-outgoing-body-chunk-size` option sets.
//!
//! Nothing here talks to the network: the body is built and then read back
//! through `check_write`, so the printed number is the configured budget.
use test_programs::wasi::http::types::{Headers, Method, OutgoingRequest, Scheme};

fn main() {
    let request = OutgoingRequest::new(Headers::new());
    request
        .set_method(&Method::Post)
        .expect("failed to set method");
    request
        .set_scheme(Some(&Scheme::Http))
        .expect("failed to set scheme");
    request
        .set_authority(Some("localhost"))
        .expect("failed to set authority");
    request
        .set_path_with_query(Some("/"))
        .expect("failed to set path_with_query");
    let body = request.body().expect("failed to get outgoing body");
    let stream = body.write().expect("failed to get output stream");
    println!("{}", stream.check_write().expect("check_write failed"));
}
