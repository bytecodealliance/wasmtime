//! Dropping an `outgoing-request` before its `headers` child is a trap.

use test_programs::wasi::http::types::{Fields, OutgoingRequest};

fn main() {
    let request = OutgoingRequest::new(Fields::new());
    let _headers = request.headers();
    drop(request);
    unreachable!("dropping outgoing-request with a live headers child should trap");
}
