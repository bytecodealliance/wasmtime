use test_programs::proxy;
use test_programs::wasi::http::types::{IncomingRequest, ResponseOutparam};

struct T;

proxy::export!(T);

impl proxy::exports::wasi::http::incoming_handler::Guest for T {
    fn handle(_request: IncomingRequest, _outparam: ResponseOutparam) {}
}

fn main() {}
